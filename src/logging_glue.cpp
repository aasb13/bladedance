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

#include <cstdint>
#include <fmt/color.h>

extern "C" {
	const char* rust_log_level_to_string(uint8_t level);

	// Manager functions
	void rust_log_manager_write(uint8_t level,
		const char* type, size_t type_length, const char* message, size_t message_length);
	void rust_log_manager_enable_debug_mode(bool forceprotodebug);
	uint8_t rust_log_manager_check_level();
	void rust_log_manager_open_logs(bool requiremethods);
	void rust_log_manager_close_logs();
	uint8_t rust_log_manager_get_maxlevel();
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

Log::Engine::Engine(Module* Creator, const std::string& Name)
	: DataProvider(Creator, "log/" + Name)
{
}

Log::Engine::~Engine()
{
	if (creator)
		::Logs.UnloadEngine(this);
}

Log::FileEngine::FileEngine(Module* Creator)
	: Engine(Creator, "file")
{
}

Log::MethodPtr Log::FileEngine::Create(const std::shared_ptr<ConfigTag>& tag)
{
	// Tracing handles output, so we don't need to create file handles
	// The configuration is still validated for compatibility
	const std::string target = tag->getString("target");
	if (target.empty())
		throw CoreException("<log:target> must be specified for file logger at " + tag->source.str());

	// Return nullptr since tracing handles all output
	return nullptr;
}

Log::StreamEngine::StreamEngine(Module* Creator, const std::string& Name)
	: Engine(Creator, Name)
{
}

Log::MethodPtr Log::StreamEngine::Create(const std::shared_ptr<ConfigTag>& tag)
{
	// Tracing handles output, so we don't need to create stream handles
	// The configuration is still validated for compatibility
	const std::string namestr = tag->getString("target", name, 1);

	// Return nullptr since tracing handles all output
	return nullptr;
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
	return level >= l && types.Contains(t);
}

Log::Manager::Manager()
	: filelog(nullptr)
{
}

void Log::Manager::CloseLogs()
{
	rust_log_manager_close_logs();
	
	// Also clean up C++ loggers
	logging = true; // Prevent writing to dying loggers.
	loggers.erase(std::remove_if(loggers.begin(), loggers.end(), [](const Info& info) { return info.config; }), loggers.end());
	logging = false;
}

void Log::Manager::EnableDebugMode()
{
	rust_log_manager_enable_debug_mode(ServerInstance->Config->CommandLine.forceprotodebug);
	
	// Update the maxlevel from Rust
	maxlevel = static_cast<Level>(rust_log_manager_get_maxlevel());
	ServerInstance->Config->RawLog = (maxlevel >= Level::RAWIO);
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

	// Parse configuration and create loggers
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

	// Call Rust to open logs and handle caching
	rust_log_manager_open_logs(requiremethods);
	
	// Update maxlevel from Rust
	maxlevel = static_cast<Level>(rust_log_manager_get_maxlevel());
	ServerInstance->Config->RawLog = (maxlevel >= Level::RAWIO);
}

void Log::Manager::RegisterServices()
{
	ServiceProvider* coreloggers[] = { &filelog };
	ServerInstance->Modules.AddServices(coreloggers, sizeof(coreloggers)/sizeof(ServiceProvider*));

	// Create stderr logger only (stdout would cause duplicate output to terminal)
	loggers.emplace_back(Level::NORMAL, TokenList("*"), CreateStreamLogger("stderr", 2), false, nullptr);
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
	uint8_t newmaxlevel = rust_log_manager_check_level();
	maxlevel = static_cast<Level>(newmaxlevel);
	ServerInstance->Config->RawLog = (maxlevel >= Level::RAWIO);
}

void Log::Manager::Write(Level level, const std::string& type, const std::string& message)
{
	rust_log_manager_write(static_cast<uint8_t>(level),
		type.c_str(), type.length(), message.c_str(), message.length());
}

Log::MethodPtr Log::Manager::CreateStreamLogger(const std::string& name, uint8_t target)
{
	// Tracing handles output, so we don't need to create stream handles
	// Return nullptr since tracing handles all output
	return nullptr;
}
