/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2022-2025 Sadie Powell <sadie@witchery.services>
 *
 * This file is part of InspIRCd.  InspIRCd is free software: you can
 * redistribute it and/or modify it under the terms of the GNU General Public
 * License as published by the Free Software Foundation, version 2.
 *
 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE.  See the GNU General Public License for more
 * details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */


#include "inspircd.h"
#include "clientprotocolmsg.h"
#include "timeutils.h"

#include <cstdint>
#include <fmt/color.h>

extern "C" {
	const char* rust_log_level_to_string(uint8_t level);

	void* rust_log_filemethod_create(const char* target, size_t target_length, unsigned long flush, uint8_t kind);
	void rust_log_filemethod_destroy(void* handle);
	StdString rust_log_filemethod_on_log(void* handle, time_t time, uint8_t level,
		const char* type, size_t type_length, const char* message, size_t message_length);
	void rust_log_filemethod_tick(void* handle);
}

const char* Log::LevelToString(Log::Level level)
{
	return rust_log_level_to_string(static_cast<uint8_t>(level));
}

void Log::NotifyRawIO(LocalUser* user, MessageType type)
{
	ClientProtocol::Messages::Privmsg msg(ServerInstance->FakeClient, user, "*** Raw I/O logging is enabled on this server. All messages, passwords, and commands are being recorded.", type);
	user->Send(ServerInstance->GetRFCEvents().privmsg, msg);
}

class DebugLogMethod final
	: public Log::Method
{
public:
	void OnLog(time_t time, Log::Level level, const std::string& type, const std::string& message) override
	{
		fmt::println("{} {}: {}",
			fmt::styled(Time::ToString(time, "%d %b %H:%M:%S"), fmt::fg(fmt::terminal_color::yellow)),
			fmt::styled(type, fmt::fg(fmt::terminal_color::green)),
			message
		);
	}
};

Log::FileMethod::FileMethod(const std::string& n, unsigned long fl, Target target)
	: Timer(15*60, true)
	, handle(nullptr)
	, flush(fl)
	, name(n)
{
	handle = rust_log_filemethod_create(name.c_str(), name.length(), flush, static_cast<uint8_t>(target));
	if (!handle)
		throw CoreException(INSP_FORMAT("Unable to create file logger for {}", name));

	if (flush > 1)
		ServerInstance->Timers.AddTimer(this);
}

Log::FileMethod::~FileMethod()
{
	rust_log_filemethod_destroy(handle);
	handle = nullptr;
}

void Log::FileMethod::OnLog(time_t time, Level level, const std::string& type, const std::string& message)
{
	StdString err = rust_log_filemethod_on_log(handle, time, static_cast<uint8_t>(level),
		type.c_str(), type.length(), message.c_str(), message.length());
	if (!err.empty())
	{
		const std::string errstr = err.to_std_string();
		StdString_Destroy(&err);
		throw CoreException(INSP_FORMAT("Unable to write to {}: {}", name, errstr));
	}
	StdString_Destroy(&err);
}

bool Log::FileMethod::Tick()
{
	rust_log_filemethod_tick(handle);
	return true;
}

Log::Engine::Engine(Module* Creator, const std::string& Name)
	: DataProvider(Creator, "log/" + Name)
{
}

Log::Engine::~Engine()
{
	if (creator)
		ServerInstance->Logs.UnloadEngine(this);
}

Log::FileEngine::FileEngine(Module* Creator)
	: Engine(Creator, "file")
{
}

Log::MethodPtr Log::FileEngine::Create(const std::shared_ptr<ConfigTag>& tag)
{
	const std::string target = tag->getString("target");
	if (target.empty())
		throw CoreException("<log:target> must be specified for file logger at " + tag->source.str());

	const std::string fulltarget = ServerInstance->Config->Paths.PrependLog(Time::ToString(ServerInstance->Time(), target.c_str()));
	const unsigned long flush = tag->getNum<unsigned long>("flush", 20, 1);
	return std::make_shared<FileMethod>(fulltarget, flush, Log::FileMethod::Target::FILE);
}

Log::StreamEngine::StreamEngine(Module* Creator, const std::string& Name, Log::FileMethod::Target t)
	: Engine(Creator, Name)
	, target(t)
{
}

Log::MethodPtr Log::StreamEngine::Create(const std::shared_ptr<ConfigTag>& tag)
{
	return std::make_shared<FileMethod>(name, 1, target);
}

Log::Manager::CachedMessage::CachedMessage(time_t ts, Level l, const std::string& t, const std::string& m)
	: time(ts)
	, level(l)
	, type(t)
	, message(m)
{
}

Log::Manager::Info::Info(Level l, TokenList t, MethodPtr m, bool c, const Engine* e)
	: config(c)
	, level(l)
	, types(std::move(t))
	, method(std::move(m))
	, engine(e)
{
}

bool Log::Manager::Info::Suitable(Level l, const std::string& t) const
{
	return level >= l && types.Contains(t) && !dead;
}

Log::Manager::Manager()
	: filelog(nullptr)
	, stderrlog(nullptr, "stderr", Log::FileMethod::Target::STDERR)
	, stdoutlog(nullptr, "stdout", Log::FileMethod::Target::STDOUT)
{
}

void Log::Manager::CloseLogs()
{
	logging = true; // Prevent writing to dying loggers.
	loggers.erase(std::remove_if(loggers.begin(), loggers.end(), [](const Info& info) { return info.config; }), loggers.end());
	logging = false;
}

void Log::Manager::EnableDebugMode()
{
	TokenList types = std::string("*");
	MethodPtr method = std::make_shared<DebugLogMethod>();

	if (ServerInstance->Config->CommandLine.forceprotodebug)
	{
		// If we are doing a protocol debug we need to warn users.
		loggers.emplace_back(Level::RAWIO, std::move(types), std::move(method), false, &stdoutlog);
		ServerInstance->Config->RawLog = true;
	}
	else
	{
		loggers.emplace_back(Level::DEBUG, std::move(types), std::move(method), false, &stdoutlog);
	}
}

void Log::Manager::OpenLogs(bool requiremethods)
{
	// If the server is started in debug mode we don't write logs.
	if (ServerInstance->Config->CommandLine.forcedebug)
	{
		const auto* option = ServerInstance->Config->CommandLine.forceprotodebug ? "--protocoldebug" : "--debug";
		Normal("LOG", "Not opening loggers because we were started with {}", option);
		CheckLevel();
		return;
	}

	// If the server is started with logging disabled we don't write logs.
	if (!ServerInstance->Config->CommandLine.writelog)
	{
		Normal("LOG", "Not opening loggers because we were started with --nolog");
		CheckLevel();
		return;
	}

	for (const auto& [_, tag] : ServerInstance->Config->ConfTags("log"))
	{
		const std::string methodstr = tag->getString("method", "file", 1);
		Log::Engine* engine = ServerInstance->Modules.FindDataService<Log::Engine>("log/" + methodstr);
		if (!engine)
		{
			if (!requiremethods)
				continue; // We will open this later.

			throw CoreException(methodstr + " is not a valid logging method at " + tag->source.str());
		}

		const Level level = tag->getEnum("level", Level::NORMAL, {
			{ "critical", Level::CRITICAL },
			{ "warning",  Level::WARNING  },
			{ "normal",   Level::NORMAL   },
			{ "debug",    Level::DEBUG    },
			{ "rawio",    Level::RAWIO    },

			// Deprecated v3 names.
			{ "sparse",  Level::CRITICAL },
			{ "verbose", Level::DEBUG    },
			{ "default", Level::NORMAL   },

		});
		TokenList types = tag->getString("type", "*", 1);
		MethodPtr method = engine->Create(tag);
		loggers.emplace_back(level, std::move(types), method, true, engine);
	}

	if (requiremethods && caching)
	{
		// The server has finished starting up so we can write out any cached log messages.
		logging = true;
		for (auto& logger : loggers)
		{
			if (logger.dead || !logger.method->AcceptsCachedMessages())
				continue; // Does not support logging.

			for (const auto& message : cache)
			{
				if (!logger.Suitable(message.level, message.type))
					continue;

				try
				{
					logger.method->OnLog(message.time, message.level, message.type, message.message);
				}
				catch (const CoreException& err)
				{
					logger.dead = true;
					logger.method.reset();
					ServerInstance->SNO.WriteGlobalSno('a', "A logger threw an exception: {}", err.GetReason());
					break;
				}
			}
		}

		cache.clear();
		cache.shrink_to_fit();
		caching = false;
		logging = false;
	}
	CheckLevel();
}

void Log::Manager::RegisterServices()
{
	ServiceProvider* coreloggers[] = { &filelog, &stderrlog, &stdoutlog };
	ServerInstance->Modules.AddServices(coreloggers, sizeof(coreloggers)/sizeof(ServiceProvider*));
}

void Log::Manager::UnloadEngine(const Engine* engine)
{
	logging = true; // Prevent writing to dying loggers.
	size_t logger_count = loggers.size();
	loggers.erase(std::remove_if(loggers.begin(), loggers.end(), [&engine](const Info& info) { return info.engine == engine; }), loggers.end());
	logging = false;

	Normal("LOG", "The {} log engine is unloading; removed {}/{} loggers.", engine->name.c_str(), logger_count - loggers.size(), logger_count);
}

void Log::Manager::CheckLevel()
{
	// There might be a logger not from the config so we need to check this outside of the creation loop.
	auto newmaxlevel = Level::LOWEST;
	for (const auto& logger : loggers)
	{
		if (logger.level > newmaxlevel)
			newmaxlevel = logger.level;
	}

	std::swap(maxlevel, newmaxlevel);
	ServerInstance->Config->RawLog = (newmaxlevel >= Level::RAWIO);

	Debug("LOG", "Changed maximum log level from {} to {}", LevelToString(newmaxlevel), LevelToString(maxlevel));
}

void Log::Manager::Write(Level level, const std::string& type, const std::string& message)
{
	if (logging)
		return; // Avoid log loops.

	if (maxlevel < level && !caching)
		return; // No loggers care about this message.

	logging = true;
	time_t time = ServerInstance->Time();
	for (auto& logger : loggers)
	{
		if (!logger.Suitable(level, type))
			continue;

		try
		{
			logger.method->OnLog(time, level, type, message);
		}
		catch (const CoreException& err)
		{
			logger.dead = true;
			logger.method.reset();
			ServerInstance->SNO.WriteGlobalSno('a', "A logger threw an exception: {}", err.GetReason());
			break;
		}
	}

	if (caching)
		cache.emplace_back(time, level, type, message);
	logging = false;
}
