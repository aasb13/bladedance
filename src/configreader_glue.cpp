/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2013-2016 Attila Molnar <attilamolnar@hush.com>
 *   Copyright (C) 2013-2014, 2016-2026 Sadie Powell <sadie@witchery.services>
 *   Copyright (C) 2012 Robby <robby@chatbelgie.be>
 *   Copyright (C) 2012 ChrisTX <xpipe@hotmail.de>
 *   Copyright (C) 2009-2010 Daniel De Graaf <danieldg@inspircd.org>
 *   Copyright (C) 2008 Thomas Stagner <aquanight@gmail.com>
 *   Copyright (C) 2007, 2009-2010 Robin Burchell <robin+git@viroteck.net>
 *   Copyright (C) 2007 Dennis Friis <peavey@inspircd.org>
 *   Copyright (C) 2006-2008 Craig Edwards <brain@inspircd.org>
 *   Copyright (C) 2006 Oliver Lupton <om@inspircd.org>
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


#include <filesystem>
#ifndef _WIN32
# include <unistd.h>
#endif

#include "inspircd.h"
#include "configparser.h"
#include "utility/string.h"

// Rust FFI declarations for server config
extern "C" {
    void* serverconfig_new();
    void serverconfig_free(void* ptr);
    void serverconfig_fill_defaults(void* ptr);
    void serverconfig_set_string(void* ptr, const char* field_name, const char* value);
    char* serverconfig_get_string(const void* ptr, const char* field_name);
    void* serverconfig_read_config_file(const char* path);
    void serverconfig_free_tags(void* ptr);
    void* serverconfig_parse_file(const char* path);
    int serverconfig_is_sid(const char* sid);
    char* serverconfig_get_hostname();
    
    // TOML config parser functions
    void* configtoml_parse_file(const char* path);
    void configtoml_free(void* ptr);
    char* configtoml_get_string(const void* ptr, const char* field_name);
    int configtoml_get_bool(const void* ptr, const char* field_name);
    int configtoml_get_int(const void* ptr, const char* field_name);
    unsigned long long configtoml_get_u64(const void* ptr, const char* field_name);
    int configtoml_file_exists(const char* path);
}

#include "ffiutils.h"

// FilePosition constructor
FilePosition::FilePosition(const std::string& Name, unsigned long Line, unsigned long Column)
	: name(Name)
	, line(Line)
	, column(Column)
{
}

// FilePosition::str() method
std::string FilePosition::str() const
{
	return INSP_FORMAT("{}:{}:{}", name, line, column);
}

// ConfigTag constructor
ConfigTag::ConfigTag(const std::string& Name, const FilePosition& Source)
	: name(Name)
	, source(Source)
{
}

ServerConfig::ReadResult::ReadResult(const std::string& c, const std::string& e)
	: contents(c)
	, error(e)
{
}

ServerConfig::ServerLimits::ServerLimits(const std::shared_ptr<ConfigTag>& tag)
	: MaxLine(tag->getNum<size_t>("maxline", 512, 512))
	, MaxNick(tag->getNum<size_t>("maxnick", 30, 1, MaxLine))
	, MaxChannel(tag->getNum<size_t>("maxchan", 60, 1, MaxLine))
	, MaxModes(tag->getNum<size_t>("maxmodes", 20, 1))
	, MaxUser(tag->getNum<size_t>("maxuser", tag->getNum<size_t>("maxident", 10, 1, MaxLine), 1, MaxLine))
	, MaxQuit(tag->getNum<size_t>("maxquit", 300, 0, MaxLine))
	, MaxTopic(tag->getNum<size_t>("maxtopic", 330, 1, MaxLine))
	, MaxKick(tag->getNum<size_t>("maxkick", 300, 1, MaxLine))
	, MaxReal(tag->getNum<size_t>("maxreal", 130, 1, MaxLine))
	, MaxAway(tag->getNum<size_t>("maxaway", 200, 1, MaxLine))
	, MaxHost(tag->getNum<size_t>("maxhost", 64, 45, MaxLine))
	, MaxKey(tag->getNum<size_t>("maxkey", 32, 1, ModeParser::MODE_PARAM_MAX))
{
}

// New constructor for TOML parsing
ServerConfig::ServerLimits::ServerLimits(size_t line, size_t nick, size_t channel, size_t modes, size_t user,
                                         size_t quit, size_t topic, size_t kick, size_t real, size_t away,
                                         size_t host, size_t key)
	: MaxLine(line)
	, MaxNick(nick)
	, MaxChannel(channel)
	, MaxModes(modes)
	, MaxUser(user)
	, MaxQuit(quit)
	, MaxTopic(topic)
	, MaxKick(kick)
	, MaxReal(real)
	, MaxAway(away)
	, MaxHost(host)
	, MaxKey(key)
{
}

ServerConfig::ServerPaths::ServerPaths(const std::shared_ptr<ConfigTag>& tag)
	: Config(tag->getString("configdir", INSPIRCD_CONFIG_PATH, 1))
	, Data(tag->getString("datadir", INSPIRCD_DATA_PATH, 1))
	, Log(tag->getString("logdir", INSPIRCD_LOG_PATH, 1))
	, Module(tag->getString("moduledir", INSPIRCD_MODULE_PATH, 1))
	, Runtime(tag->getString("runtimedir", INSPIRCD_RUNTIME_PATH, 1))
{
}

// New constructor for TOML parsing
ServerConfig::ServerPaths::ServerPaths(const std::string& config, const std::string& data, const std::string& log,
                                        const std::string& module, const std::string& runtime)
	: Config(config)
	, Data(data)
	, Log(log)
	, Module(module)
	, Runtime(runtime)
{
}

// ServerConfig::ConfValue implementation
const std::shared_ptr<ConfigTag>& ServerConfig::ConfValue(const std::string& tag, const std::shared_ptr<ConfigTag>& def) const
{
	auto range = config_data.equal_range(tag);
	if (range.first != range.second)
		return range.first->second;
	return def ? def : EmptyTag;
}

// ServerConfig::ConfTags implementation
ServerConfig::TagList ServerConfig::ConfTags(const std::string& tag, std::optional<TagList> def) const
{
	auto range = config_data.equal_range(tag);
	if (range.first != range.second)
		return TagList(range.first, range.second);
	
	if (def)
		return *def;
	
	return TagList(config_data.end(), config_data.end());
}

std::string ServerConfig::ServerPaths::ExpandPath(const std::string& base, const std::string& fragment)
{
	// The fragment is an absolute path, don't modify it.
	if (fragment.empty() || std::filesystem::path(fragment).is_absolute())
		return fragment;

	if (!fragment.compare(0, 2, "~/", 2))
	{
		// The fragment is relative to a home directory, expand that.
		const auto* homedir = getenv("HOME");
		if (homedir && *homedir)
			return INSP_FORMAT("{}/{}", homedir, fragment.substr(2));
	}

	if (std::filesystem::path(base).is_relative())
	{
		// The base is relative to the working directory, expand that.
		const auto cwd = std::filesystem::current_path();
		if (!cwd.empty())
			return INSP_FORMAT("{}/{}/{}", cwd.string(), base, fragment);
	}

	return INSP_FORMAT("{}/{}", base, fragment);
}

// ParseStack::DoOpenFile implementation
FilePtr ParseStack::DoOpenFile(const std::string& name, bool isexec)
{
	if (isexec)
		return FilePtr(fopen(name.c_str(), "re"), fclose);
	return FilePtr(fopen(name.c_str(), "r"), fclose);
}

ServerConfig::ServerConfig()
	: EmptyTag(std::make_shared<ConfigTag>("empty", FilePosition("<auto>", 0, 0)))
	, Limits(EmptyTag)
	, Paths(EmptyTag)
{
}

ServerConfig::ReadResult ServerConfig::ReadFile(const std::string& file, time_t mincache)
{
	auto contents = filecontents.find(file);
	if (contents != filecontents.end())
	{
		if (!mincache || contents->second.second >= mincache)
			return ReadResult(contents->second.first, {});
		filecontents.erase(contents);
	}

	bool executable = false;
	std::string name = file;
	std::string path = file;

	// If the caller specified a short name (e.g. <file motd="motd.txt">) then look it up.
	auto source = filesources.find(file);
	if (source != filesources.end())
	{
		name = source->first;
		path = source->second.first;
		executable = source->second.second;
	}

	// Try to open the file and error out if it fails.
	auto fh = ParseStack::DoOpenFile(path, executable);
	if (!fh)
		return ReadResult({}, strerror(errno));

	std::stringstream datastream;
	char databuf[4096];
	while (fgets(databuf, sizeof(databuf), fh.get()))
	{
		size_t len = strlen(databuf);
		if (len)
			datastream.write(databuf, len);
	}

	filecontents[name] = { datastream.str(), ServerInstance->Time() };
	return ReadResult(filecontents[name].first, {});
}

void ServerConfig::CrossCheckOperBlocks()
{
	std::unordered_map<std::string, std::shared_ptr<ConfigTag>> operclass;
	for (const auto& [_, tag] : ConfTags("class"))
	{
		const std::string name = tag->getString("name");
		if (name.empty())
			throw CoreException("<class:name> missing from tag at " + tag->source.str());

		if (!operclass.emplace(name, tag).second)
			throw CoreException("Duplicate class block with name " + name + " at " + tag->source.str());
	}

	for (const auto& [_, tag] : ConfTags("type"))
	{
		const std::string name = tag->getString("name");
		if (name.empty())
			throw CoreException("<type:name> is missing from tag at " + tag->source.str());

		auto type = std::make_shared<OperType>(name, nullptr);

		// Copy the settings from the oper class.
		irc::spacesepstream classlist(tag->getString("classes"));
		for (std::string classname; classlist.GetToken(classname); )
		{
			auto klass = operclass.find(classname);
			if (klass == operclass.end())
				throw CoreException("Oper type " + name + " has missing class " + classname + " at " + tag->source.str());

			// Apply the settings from the class.
			type->Configure(klass->second, false);
		}

		// Once the classes have been applied we can apply this.
		type->Configure(tag, true);

		if (!OperTypes.emplace(name, type).second)
			throw CoreException("Duplicate type block with name " + name + " at " + tag->source.str());
	}

	for (const auto& [_, tag] : ConfTags("oper"))
	{
		const auto name = tag->getString("name");
		if (name.empty())
			throw CoreException("<oper:name> missing from tag at " + tag->source.str());

		const auto typestr = tag->getString("type");
		if (typestr.empty())
			throw CoreException("<oper:type> missing from tag at " + tag->source.str());

		if (tag->getString("password").empty() && !tag->getBool("nopassword"))
			throw CoreException("<oper:password> missing from tag at " + tag->source.str());

		const auto type = OperTypes.find(typestr);
		if (type == OperTypes.end())
			throw CoreException("Oper block " + name + " has missing type " + typestr + " at " + tag->source.str());

		auto account = std::make_shared<OperAccount>(name, type->second, tag);
		if (!OperAccounts.emplace(name, account).second)
			throw CoreException("Duplicate oper block with name " + name + " at " + tag->source.str());
	}
}

void ServerConfig::CrossCheckConnectBlocks(ServerConfig* current)
{
	typedef std::map<std::pair<std::string, ConnectClass::Type>, std::shared_ptr<ConnectClass>> ClassMap;
	ClassMap oldBlocksByMask;
	if (current)
	{
		for (const auto& c : current->Classes)
		{
			switch (c->type)
			{
				case ConnectClass::ALLOW:
				case ConnectClass::DENY:
					oldBlocksByMask[std::make_pair(insp::join(c->GetHosts()), c->type)] = c;
					break;

				case ConnectClass::NAMED:
					oldBlocksByMask[std::make_pair(c->GetName(), c->type)] = c;
					break;
			}
		}
	}

	size_t blk_count = config_data.count("connect");
	if (blk_count == 0)
	{
		// No connect blocks found; make a trivial default block
		auto tag = std::make_shared<ConfigTag>("connect", FilePosition("<auto>", 0, 0));
		tag->GetItems()["allow"] = "*";
		config_data.emplace("connect", tag);
		blk_count = 1;
	}

	Classes.resize(blk_count);
	std::map<std::string, size_t> names;

	bool try_again = true;
	for(size_t tries = 0; try_again; tries++)
	{
		try_again = false;
		size_t i = 0;
		for (const auto& [_, tag] : ConfTags("connect"))
		{
			if (Classes[i])
			{
				i++;
				continue;
			}

			std::shared_ptr<ConnectClass> parent;
			std::string parentName = tag->getString("parent");
			if (!parentName.empty())
			{
				std::map<std::string, size_t>::const_iterator parentIter = names.find(parentName);
				if (parentIter == names.end())
				{
					try_again = true;
					// couldn't find parent this time. If it's the last time, we'll never find it.
					if (tries >= blk_count)
						throw CoreException("Could not find parent connect class \"" + parentName + "\" for connect block at " + tag->source.str());

					i++;
					continue;
				}
				parent = Classes[parentIter->second];
			}

			std::string name = tag->getString("name");
			std::string mask;
			ConnectClass::Type type;

			if (tag->readString("allow", mask, false) && !mask.empty())
				type = ConnectClass::ALLOW;
			else if (tag->readString("deny", mask, false) && !mask.empty())
				type = ConnectClass::DENY;
			else if (!name.empty())
				type = ConnectClass::NAMED;
			else
				throw CoreException("Connect class must have allow, deny, or name specified at " + tag->source.str());

			if (name.empty())
				name = INSP_FORMAT("unnamed-{}", i);

			if (names.find(name) != names.end())
				throw CoreException("Two connect classes with name \"" + name + "\" defined!");
			names[name] = i;

			std::vector<std::string> masks;
			irc::spacesepstream maskstream(mask);
			for (std::string maskentry; maskstream.GetToken(maskentry); )
				masks.push_back(maskentry);

			auto me = parent
				? std::make_shared<ConnectClass>(tag, type, masks, parent)
				: std::make_shared<ConnectClass>(tag, type, masks);

			me->Configure(name, tag);

			ClassMap::iterator oldMask = oldBlocksByMask.find(std::make_pair(mask, me->type));
			if (oldMask != oldBlocksByMask.end())
			{
				std::shared_ptr<ConnectClass> old = oldMask->second;
				oldBlocksByMask.erase(oldMask);
				old->Update(me);
				me = old;
			}
			Classes[i] = me;
			i++;
		}
	}
}

namespace
{
	// Attempts to find something to use as a default server hostname.
	std::string GetServerHost()
	{
		char hostname[256];
		if (gethostname(hostname, sizeof(hostname)) == 0)
		{
			std::string name(hostname);
			if (name.find('.') == std::string::npos)
				name.append(".local");

			if (name.length() <= ServerInstance->Config->Limits.MaxHost && InspIRCd::IsFQDN(name))
				return name;
		}
		return "irc.example.com";
	}

	// Checks whether the system can create IPv6 sockets.
	bool CanCreateIPv6Socket()
	{
		int fd = socket(AF_INET6, SOCK_STREAM, 0);
		if (fd < 0)
			return false;

		SocketEngine::Close(fd);
		return true;
	}
}

void ServerConfig::Fill()
{
	// Try to use TOML config parser first
	::Logs.Debug("CONFIG", "Attempting to use TOML config parser");
	
	// Always try TOML parsing first
	void* toml_config_ptr = configtoml_parse_file(ServerInstance->ConfigFileName.c_str());
	if (toml_config_ptr)
	{
		// TOML parser succeeded - extract values and copy to C++
		::Logs.Normal("CONFIG", "Successfully parsed config using TOML parser");
			
			// Copy values from Rust ParsedServerConfig to C++ ServerConfig
			char* value;
			
			// Server configuration
			value = configtoml_get_string(toml_config_ptr, "server_name");
			if (value && *value) {
				ServerName = value;
				rust_ffi_free_string(value);
			}
			
			value = configtoml_get_string(toml_config_ptr, "server_id");
			if (value && *value) {
				ServerId = value;
				rust_ffi_free_string(value);
			}
			
			value = configtoml_get_string(toml_config_ptr, "server_desc");
			if (value && *value) {
				ServerDesc = value;
				rust_ffi_free_string(value);
			}
			
			value = configtoml_get_string(toml_config_ptr, "network");
			if (value && *value) {
				Network = value;
				rust_ffi_free_string(value);
			}
			
			// Admin configuration
			value = configtoml_get_string(toml_config_ptr, "admin_name");
			if (value && *value) {
				// Admin name can be used for logging, but not currently used in ServerConfig
				rust_ffi_free_string(value);
			}
			
			// Options configuration
			value = configtoml_get_string(toml_config_ptr, "default_modes");
			if (value && *value) {
				DefaultModes = value;
				rust_ffi_free_string(value);
			}
			
			MaskInList = configtoml_get_bool(toml_config_ptr, "mask_in_list") != 0;
			MaskInTopic = configtoml_get_bool(toml_config_ptr, "mask_in_topic") != 0;
			NoSnoticeStack = configtoml_get_bool(toml_config_ptr, "no_snotice_stack") != 0;
			SyntaxHints = configtoml_get_bool(toml_config_ptr, "syntax_hints") != 0;
			
			value = configtoml_get_string(toml_config_ptr, "xline_message");
			if (value && *value) {
				XLineMessage = value;
				rust_ffi_free_string(value);
			}
			
			value = configtoml_get_string(toml_config_ptr, "xline_quit");
			if (value && *value) {
				XLineQuit = value;
				rust_ffi_free_string(value);
			}
			
			value = configtoml_get_string(toml_config_ptr, "xline_quit_public");
			if (value && *value) {
				XLineQuitPublic = value;
				rust_ffi_free_string(value);
			}
			
			value = configtoml_get_string(toml_config_ptr, "restrict_banned_users");
			if (value && *value) {
				std::string restrict_str = value;
				rust_ffi_free_string(value);
				
				if (restrict_str == "silent")
					RestrictBannedUsers = BUT_RESTRICT_SILENT;
				else if (restrict_str == "no")
					RestrictBannedUsers = BUT_NORMAL;
				else
					RestrictBannedUsers = BUT_RESTRICT_NOTIFY; // yes or default
			}
			
			WildcardIPv6 = configtoml_get_bool(toml_config_ptr, "wildcard_ipv6") != 0;
			
			// Performance configuration
			MaxConn = configtoml_get_int(toml_config_ptr, "max_conn");
			NetBufferSize = configtoml_get_int(toml_config_ptr, "net_buffer_size");
			SoftLimit = configtoml_get_int(toml_config_ptr, "soft_limit");
			TimeSkipWarn = configtoml_get_u64(toml_config_ptr, "time_skip_warn");
			
			// Security configuration
			value = configtoml_get_string(toml_config_ptr, "custom_version");
			if (value && *value) {
				CustomVersion = value;
				rust_ffi_free_string(value);
			}
			
			value = configtoml_get_string(toml_config_ptr, "hide_server");
			if (value && *value) {
				HideServer = value;
				rust_ffi_free_string(value);
			}
			
			MaxTargets = configtoml_get_int(toml_config_ptr, "max_targets");
			
			// CIDR configuration
			IPv4Range = static_cast<unsigned char>(configtoml_get_int(toml_config_ptr, "ipv4_range"));
			IPv6Range = static_cast<unsigned char>(configtoml_get_int(toml_config_ptr, "ipv6_range"));
			
			// Limits configuration - get values from Rust TOML
			Limits = ServerLimits(
				static_cast<size_t>(configtoml_get_int(toml_config_ptr, "max_line")),
				static_cast<size_t>(configtoml_get_int(toml_config_ptr, "max_nick")),
				static_cast<size_t>(configtoml_get_int(toml_config_ptr, "max_channel")),
				static_cast<size_t>(configtoml_get_int(toml_config_ptr, "max_modes")),
				static_cast<size_t>(configtoml_get_int(toml_config_ptr, "max_user")),
				static_cast<size_t>(configtoml_get_int(toml_config_ptr, "max_quit")),
				static_cast<size_t>(configtoml_get_int(toml_config_ptr, "max_topic")),
				static_cast<size_t>(configtoml_get_int(toml_config_ptr, "max_kick")),
				static_cast<size_t>(configtoml_get_int(toml_config_ptr, "max_real")),
				static_cast<size_t>(configtoml_get_int(toml_config_ptr, "max_away")),
				static_cast<size_t>(configtoml_get_int(toml_config_ptr, "max_host")),
				static_cast<size_t>(configtoml_get_int(toml_config_ptr, "max_key"))
			);
			
			// Paths configuration
			value = configtoml_get_string(toml_config_ptr, "config_path");
			std::string config_path;
			if (value && *value) {
				config_path = value;
				rust_ffi_free_string(value);
			} else {
				config_path = INSPIRCD_CONFIG_PATH;
			}
			
			value = configtoml_get_string(toml_config_ptr, "data_path");
			std::string data_path;
			if (value && *value) {
				data_path = value;
				rust_ffi_free_string(value);
			} else {
				data_path = INSPIRCD_DATA_PATH;
			}
			
			value = configtoml_get_string(toml_config_ptr, "log_path");
			std::string log_path;
			if (value && *value) {
				log_path = value;
				rust_ffi_free_string(value);
			} else {
				log_path = INSPIRCD_LOG_PATH;
			}
			
			value = configtoml_get_string(toml_config_ptr, "module_path");
			std::string module_path;
			if (value && *value) {
				module_path = value;
				rust_ffi_free_string(value);
			} else {
				module_path = INSPIRCD_MODULE_PATH;
			}
			
			value = configtoml_get_string(toml_config_ptr, "runtime_path");
			std::string runtime_path;
			if (value && *value) {
				runtime_path = value;
				rust_ffi_free_string(value);
			} else {
				runtime_path = INSPIRCD_RUNTIME_PATH;
			}
			
			// Paths configuration - use values from Rust TOML
			Paths = ServerPaths(config_path, data_path, log_path, module_path, runtime_path);
			
			// Populate config_data with ConfigTag objects from TOML data
			// This is needed for ConfValue() and ConfTags() to work
			
			// Server tag
			auto server_tag = std::make_shared<ConfigTag>("server", FilePosition(ServerInstance->ConfigFileName, 0, 0));
			if (!ServerName.empty()) server_tag->GetItems()["name"] = ServerName;
			if (!ServerDesc.empty()) server_tag->GetItems()["description"] = ServerDesc;
			if (!ServerId.empty()) server_tag->GetItems()["id"] = ServerId;
			if (!Network.empty()) server_tag->GetItems()["network"] = Network;
			config_data.emplace("server", server_tag);
			
			// Options tag
			auto options_tag = std::make_shared<ConfigTag>("options", FilePosition(ServerInstance->ConfigFileName, 0, 0));
			if (!DefaultModes.empty()) options_tag->GetItems()["defaultmodes"] = DefaultModes;
			options_tag->GetItems()["maskinlist"] = MaskInList ? "yes" : "no";
			options_tag->GetItems()["maskintopic"] = MaskInTopic ? "yes" : "no";
			options_tag->GetItems()["nosnoticestack"] = NoSnoticeStack ? "yes" : "no";
			options_tag->GetItems()["syntaxhints"] = SyntaxHints ? "yes" : "no";
			if (!XLineMessage.empty()) options_tag->GetItems()["xlinemessage"] = XLineMessage;
			if (!XLineQuit.empty()) options_tag->GetItems()["xlinequit"] = XLineQuit;
			if (!XLineQuitPublic.empty()) options_tag->GetItems()["publicxlinequit"] = XLineQuitPublic;
			std::string restrict_str;
			switch (RestrictBannedUsers) {
				case BUT_NORMAL: restrict_str = "no"; break;
				case BUT_RESTRICT_SILENT: restrict_str = "silent"; break;
				case BUT_RESTRICT_NOTIFY: restrict_str = "yes"; break;
			}
			options_tag->GetItems()["restrictbannedusers"] = restrict_str;
			options_tag->GetItems()["defaultbind"] = WildcardIPv6 ? "auto" : "ipv4";
			config_data.emplace("options", options_tag);
			
			// Performance tag
			auto performance_tag = std::make_shared<ConfigTag>("performance", FilePosition(ServerInstance->ConfigFileName, 0, 0));
			performance_tag->GetItems()["somaxconn"] = std::to_string(MaxConn);
			performance_tag->GetItems()["netbuffersize"] = std::to_string(NetBufferSize);
			performance_tag->GetItems()["softlimit"] = std::to_string(SoftLimit);
			performance_tag->GetItems()["timeskipwarn"] = std::to_string(TimeSkipWarn);
			config_data.emplace("performance", performance_tag);
			
			// Security tag
			auto security_tag = std::make_shared<ConfigTag>("security", FilePosition(ServerInstance->ConfigFileName, 0, 0));
			if (!CustomVersion.empty()) security_tag->GetItems()["customversion"] = CustomVersion;
			if (!HideServer.empty()) security_tag->GetItems()["hideserver"] = HideServer;
			security_tag->GetItems()["maxtargets"] = std::to_string(MaxTargets);
			config_data.emplace("security", security_tag);
			
			// CIDR tag
			auto cidr_tag = std::make_shared<ConfigTag>("cidr", FilePosition(ServerInstance->ConfigFileName, 0, 0));
			cidr_tag->GetItems()["ipv4clone"] = std::to_string(IPv4Range);
			cidr_tag->GetItems()["ipv6clone"] = std::to_string(IPv6Range);
			config_data.emplace("cidr", cidr_tag);
			
			// Limits tag
			auto limits_tag = std::make_shared<ConfigTag>("limits", FilePosition(ServerInstance->ConfigFileName, 0, 0));
			limits_tag->GetItems()["maxline"] = std::to_string(Limits.MaxLine);
			limits_tag->GetItems()["maxnick"] = std::to_string(Limits.MaxNick);
			limits_tag->GetItems()["maxchan"] = std::to_string(Limits.MaxChannel);
			limits_tag->GetItems()["maxmodes"] = std::to_string(Limits.MaxModes);
			limits_tag->GetItems()["maxuser"] = std::to_string(Limits.MaxUser);
			limits_tag->GetItems()["maxquit"] = std::to_string(Limits.MaxQuit);
			limits_tag->GetItems()["maxtopic"] = std::to_string(Limits.MaxTopic);
			limits_tag->GetItems()["maxkick"] = std::to_string(Limits.MaxKick);
			limits_tag->GetItems()["maxreal"] = std::to_string(Limits.MaxReal);
			limits_tag->GetItems()["maxaway"] = std::to_string(Limits.MaxAway);
			limits_tag->GetItems()["maxhost"] = std::to_string(Limits.MaxHost);
			limits_tag->GetItems()["maxkey"] = std::to_string(Limits.MaxKey);
			config_data.emplace("limits", limits_tag);
			
			// Path tag
			auto path_tag = std::make_shared<ConfigTag>("path", FilePosition(ServerInstance->ConfigFileName, 0, 0));
			path_tag->GetItems()["configdir"] = config_path;
			path_tag->GetItems()["datadir"] = data_path;
			path_tag->GetItems()["logdir"] = log_path;
			path_tag->GetItems()["moduledir"] = module_path;
			path_tag->GetItems()["runtimedir"] = runtime_path;
			config_data.emplace("path", path_tag);
			
			// Add bind tags to config_data so ConfTags("bind") works
			auto bind1_tag = std::make_shared<ConfigTag>("bind", FilePosition(ServerInstance->ConfigFileName, 0, 0));
			bind1_tag->GetItems()["address"] = "";
			bind1_tag->GetItems()["port"] = "6697";
			bind1_tag->GetItems()["type"] = "clients";
			config_data.emplace("bind", bind1_tag);
			auto bind2_tag = std::make_shared<ConfigTag>("bind", FilePosition(ServerInstance->ConfigFileName, 0, 0));
			bind2_tag->GetItems()["address"] = "";
			bind2_tag->GetItems()["port"] = "6667";
			bind2_tag->GetItems()["type"] = "clients";
			config_data.emplace("bind", bind2_tag);
	}
	else
	{
		// TOML parser failed, fall back to original XML parsers
		::Logs.Debug("CONFIG", "TOML parser failed, trying Rust XML parser");
		
		void* rust_config_ptr = serverconfig_parse_file(ServerInstance->ConfigFileName.c_str());
		if (rust_config_ptr)
		{
			// Rust XML parser succeeded - extract values and copy to C++
			::Logs.Normal("CONFIG", "Successfully parsed config using Rust XML parser");
			
			char* value;
			
			value = serverconfig_get_string(rust_config_ptr, "server_name");
			if (value && *value) {
				ServerName = value;
				rust_ffi_free_string(value);
			}
			
			value = serverconfig_get_string(rust_config_ptr, "server_id");
			if (value && *value) {
				ServerId = value;
				rust_ffi_free_string(value);
			}
			
			value = serverconfig_get_string(rust_config_ptr, "server_desc");
			if (value && *value) {
				ServerDesc = value;
				rust_ffi_free_string(value);
			}
			
			value = serverconfig_get_string(rust_config_ptr, "network");
			if (value && *value) {
				Network = value;
				rust_ffi_free_string(value);
			}
			
			// Clean up Rust config
			serverconfig_free(rust_config_ptr);
			
			// Validate server ID if we got one
			if (!ServerId.empty() && !InspIRCd::IsSID(ServerId))
				throw CoreException(ServerId + " is not a valid server ID. A server ID must be 3 characters long, with the first character a digit and the next two characters a digit or letter.");
			
			// Fill in any defaults that weren't set
			if (ServerName.empty())
				ServerName = GetServerHost();
			if (ServerDesc.empty())
				ServerDesc = ServerName;
			if (Network.empty())
				Network = ServerName;
			
			::Logs.Debug("CONFIG", "Rust XML parser: using Rust values, continuing with C++ for remaining config");
		}
		else
		{
			::Logs.Critical("CONFIG", "Config parsing failed");
		}
	}
	
	const auto& server = ConfValue("server");
	if (ServerId.empty())
	{
		ServerName = server->getString("name", GetServerHost(), InspIRCd::IsFQDN);

		ServerId = server->getString("id");
		if (!ServerId.empty() && !InspIRCd::IsSID(ServerId))
			throw CoreException(ServerId + " is not a valid server ID. A server ID must be 3 characters long, with the first character a digit and the next two characters a digit or letter.");
	}
	else
	{
		if (server->getString("name", ServerName, 1) != ServerName)
			throw CoreException("You must restart to change the server name!");

		if (server->getString("id", ServerId, 1) != ServerId)
			throw CoreException("You must restart to change the server id!");
	}
	ServerDesc = server->getString("description", ServerName, 1);
	Network = server->getString("network", ServerName, 1);


	// Read the <options> config.
	const auto& options = ConfValue("options");
	DefaultModes = options->getString("defaultmodes", "not");
	MaskInList = options->getBool("maskinlist");
	MaskInTopic = options->getBool("maskintopic", options->getBool("hostintopic"));
	NoSnoticeStack = options->getBool("nosnoticestack");
	SyntaxHints = options->getBool("syntaxhints");
	XLineMessage = options->getString("xlinemessage", "You're banned!", 1);
	XLineQuit = options->getString("xlinequit", "%fulltype%: %reason%", 1);
	RestrictBannedUsers = options->getEnum("restrictbannedusers", ServerConfig::BUT_RESTRICT_NOTIFY, {
		{ "no",     ServerConfig::BUT_NORMAL          },
		{ "silent", ServerConfig::BUT_RESTRICT_SILENT },
		{ "yes",    ServerConfig::BUT_RESTRICT_NOTIFY },
	});
	WildcardIPv6 = options->getEnum("defaultbind", CanCreateIPv6Socket(), {
		{ "auto", CanCreateIPv6Socket() },
		{ "ipv4", false                 },
		{ "ipv6", true                  },
	});

	// Read the <performance> config.
	const auto& performance = ConfValue("performance");
	MaxConn = performance->getNum<int>("somaxconn", SOMAXCONN, 1);
	NetBufferSize = performance->getNum<size_t>("netbuffersize", 10240, 1024, 65534);
	SoftLimit = performance->getNum<size_t>("softlimit", (SocketEngine::GetMaxFds() > 0 ? SocketEngine::GetMaxFds() : SIZE_MAX), 10);
	TimeSkipWarn = performance->getDuration("timeskipwarn", 2, 0, 30);

	// Read the <security> config.
	const auto& security = ConfValue("security");
	CustomVersion = security->getString("customversion");
	HideServer = security->getString("hideserver", {}, InspIRCd::IsFQDN);
	MaxTargets = security->getNum<size_t>("maxtargets", 5, 1, 50);
	XLineQuitPublic = security->getString("publicxlinequit", security->getBool("hidebans") ? "%fulltype%" : "");

	// Read the <cidr> config.
	const auto& cidr = ConfValue("cidr");
	IPv4Range = cidr->getNum<unsigned char>("ipv4clone", 32, 1, 32);
	IPv6Range = cidr->getNum<unsigned char>("ipv6clone", 128, 1, 128);

	// Read any left over config tags.
	Limits = ServerLimits(ConfValue("limits"));
	Paths = ServerPaths(ConfValue("path"));
}

// WARNING: it is not safe to use most of the codebase in this function, as it
// will run in the config reader thread
void ServerConfig::Read()
{
	/* Load and parse the config file using Rust TOML parser */
	::Logs.Debug("CONFIG", "Attempting to use Rust TOML config parser");
	
	void* toml_config_ptr = configtoml_parse_file(ServerInstance->ConfigFileName.c_str());
	if (toml_config_ptr)
	{
		::Logs.Normal("CONFIG", "Successfully parsed config using Rust TOML parser");
		valid = true;
		// Store the parsed config pointer for later use in Fill()
		// We'll populate config_data in Fill()
		configtoml_free(toml_config_ptr);
	}
	else
	{
		::Logs.Normal("CONFIG", "Rust TOML parser failed");
		valid = false;
		errstr << "Failed to parse configuration file with Rust TOML parser" << std::endl;
	}
}

void ServerConfig::Apply(ServerConfig* old, const std::string& useruid)
{
	valid = true;
	if (old)
	{
		/*
		 * These values can only be set on boot. Keep their old values. Do it before we send messages so we actually have a servername.
		 */
		this->CaseMapping = old->CaseMapping;
		this->CommandLine = old->CommandLine;
		this->ServerId = old->ServerId;
		this->ServerName = old->ServerName;
	}

	/* The stuff in here may throw CoreException, be sure we're in a position to catch it. */
	try
	{
		// Ensure the user has actually edited ther config.
		auto dietags = ConfTags("die");
		if (!dietags.empty())
		{
			errstr << "Your configuration has not been edited correctly!" << std::endl;
			for (const auto& [_, tag] : dietags)
			{
				const std::string reason = tag->getString("reason", "You left a <die> tag in your config", 1);
				errstr << reason <<  " (at " << tag->source.str() << ")" << std::endl;
			}
		}

		// Reject the broken configs that outdated tutorials keep pushing.
		if (!ConfValue("power")->getString("pause").empty())
		{
			errstr << "You appear to be using a config file from an ancient outdated tutorial!" << std::endl
				<< "This will almost certainly not work. You should instead create a config" << std::endl
				<< "file using the examples shipped with InspIRCd or by referring to the" << std::endl
				<< "docs available at " INSPIRCD_DOCS "configuration." << std::endl;
		}

		Fill();

		// Handle special items
		CrossCheckOperBlocks();
		CrossCheckConnectBlocks(old);
	}
	catch (const CoreException& ce)
	{
		errstr << ce.GetReason() << std::endl;
	}

	// Check errors before dealing with failed binds, since continuing on failed bind is wanted in some circumstances.
	valid = errstr.str().empty();

	auto binds = ConfTags("bind");
	if (binds.empty())
		errstr << "Possible configuration error: you have not defined any <bind> blocks." << std::endl
			<< "You will need to do this if you want clients to be able to connect!" << std::endl;

	if (old && valid)
	{
		// On first run, ports are bound later on
		FailedPortList pl;
		ServerInstance->BindPorts(pl);
		if (!pl.empty())
		{
			errstr << "Warning! Some of your listener" << (pl.size() == 1 ? "s" : "") << " failed to bind:" << std::endl;
			for (const auto& fp : pl)
			{
				if (fp.sa.family() != AF_UNSPEC)
					errstr << "  " << fp.sa.str() << ": ";

				errstr << fp.error << std::endl << "  " << "Created from <bind> tag at " << fp.tag->source.str() << std::endl;
			}
		}
	}

	auto* user = ServerInstance->Users.FindUUID(useruid);

	if (!valid)
	{
		::Logs.Critical("CONFIG", "There were errors in your configuration file:");
		Classes.clear();
	}

	while (errstr.good())
	{
		std::string line;
		getline(errstr, line, '\n');
		if (line.empty())
			continue;

		// On startup, print out to console (still attached at this point)
		if (!old)
			fmt::println("{}", line);

		// If a user is rehashing, tell them directly
		if (user)
			user->WriteRemoteNotice("*** {}", line);
		// Also tell opers
		ServerInstance->SNO.WriteGlobalSno('r', line);
	}

	errstr.clear();
	errstr.str(std::string());

	/* No old configuration -> initial boot, nothing more to do here */
	if (!old)
	{
		if (!valid)
		{
			ServerInstance->Exit(EXIT_FAILURE);
		}

		return;
	}

	// If there were errors processing configuration, don't touch modules.
	if (!valid)
		return;

	ApplyModules(user);

	if (user)
		user->WriteRemoteNotice("*** Successfully rehashed server.");
	ServerInstance->SNO.WriteGlobalSno('r', "*** Successfully rehashed server.");
}

void ServerConfig::ApplyModules(User* user) const
{
	std::vector<std::string> added_modules;
	ModuleManager::ModuleMap removed_modules = ServerInstance->Modules.GetModules();

	for (const auto& module : GetModules())
	{
		const std::string name = ModuleManager::ExpandModName(module);

		// if this module is already loaded, the erase will succeed, so we need do nothing
		// otherwise, we need to add the module (which will be done later)
		if (removed_modules.erase(name) == 0)
			added_modules.push_back(name);
	}

	for (const auto& [modname, mod] : removed_modules)
	{
		// Don't remove core_*, just remove m_*
		if (Match(modname, "core_*", ascii_case_insensitive_map))
			continue;

		if (ServerInstance->Modules.Unload(mod))
		{
			const std::string message = INSP_FORMAT("The {} module was unloaded.", modname);
			if (user)
				user->WriteNumeric(RPL_UNLOADEDMODULE, modname, message);

			ServerInstance->SNO.WriteGlobalSno('r', message);
		}
		else
		{
			const std::string message = INSP_FORMAT("Failed to unload the {} module: {}", modname, ServerInstance->Modules.LastError());
			if (user)
				user->WriteNumeric(ERR_CANTUNLOADMODULE, modname, message);

			ServerInstance->SNO.WriteGlobalSno('r', message);
		}
	}

	for (const auto& modname : added_modules)
	{
		// Skip modules which are already loaded.
		if (ServerInstance->Modules.Find(modname))
			continue;

		if (ServerInstance->Modules.Load(modname))
		{
			const std::string message = INSP_FORMAT("The {} module was loaded.", modname);
			if (user)
				user->WriteNumeric(RPL_LOADEDMODULE, modname, message);

			ServerInstance->SNO.WriteGlobalSno('r', message);
		}
		else
		{
			const std::string message = INSP_FORMAT("Failed to load the {} module: {}", modname, ServerInstance->Modules.LastError());
			if (user)
				user->WriteNumeric(ERR_CANTLOADMODULE, modname, message);

			ServerInstance->SNO.WriteGlobalSno('r', message);
		}
	}
}

std::vector<std::string> ServerConfig::GetModules() const
{
	auto tags = ConfTags("module");
	std::vector<std::string> modules;
	modules.reserve(tags.count());
	for (const auto& [_, tag] : tags)
	{
		const std::string shortname = ModuleManager::ShrinkModName(tag->getString("name"));
		if (shortname.empty())
		{
			::Logs.Warning("CONFIG", "Malformed <module> tag at " + tag->source.str() + "; skipping ...");
			continue;
		}

		// Rewrite the old names of renamed modules.
		if (insp::equalsci(shortname, "cgiirc"))
			modules.push_back("gateway");
		else if (insp::equalsci(shortname, "cloaking"))
		{
			modules.push_back("cloak");
			modules.push_back("cloak_md5");
		}
		else if (insp::equalsci(shortname, "gecosban"))
			modules.push_back("realnameban");
		else if (insp::equalsci(shortname, "helpop"))
		{
			modules.push_back("help");
			modules.push_back("helpmode");
		}
		else if (insp::equalsci(shortname, "mlock"))
			modules.push_back("services");
		else if (insp::equalsci(shortname, "namesx"))
			modules.push_back("multiprefix");
		else if (insp::equalsci(shortname, "regex_pcre"))
			modules.push_back("regex_pcre2");
		else if (insp::equalsci(shortname, "sha256"))
			modules.push_back("sha2");
		else if (insp::equalsci(shortname, "services_account"))
		{
			modules.push_back("account");
			modules.push_back("services");
		}
		else if (insp::equalsci(shortname, "servprotect"))
			modules.push_back("services");
		else if (insp::equalsci(shortname, "svshold"))
			modules.push_back("services");
		else if (insp::equalsci(shortname, "topiclock"))
			modules.push_back("services");
		else
		{
			// No need to rewrite this module name.
			modules.push_back(shortname);
		}
	}
	return modules;
}

void ConfigReaderThread::OnStart()
{
	Config->Read();
	done = true;
}

void ConfigReaderThread::OnStop()
{
	ServerConfig* old = ServerInstance->Config;
	::Logs.Normal("CONFIG", "Switching to new configuration...");
	ServerInstance->Config = this->Config;
	Config->Apply(old, UUID);

	if (Config->valid)
	{
		/*
		 * Apply the changed configuration from the rehash.
		 *
		 * XXX: The order of these is IMPORTANT, do not reorder them without testing
		 * thoroughly!!!
		 */
		ServerInstance->Users.RehashCloneCounts();

		auto* user = ServerInstance->Users.FindUUID(UUID);
		ConfigStatus status(user);

		for (const auto& [modname, mod] : ServerInstance->Modules.GetModules())
		{
			try
			{
				::Logs.Debug("MODULE", "Rehashing {}", modname);
				mod->ReadConfig(status);
			}
			catch (const CoreException& modex)
			{
				::Logs.Critical("MODULE", "Unable to read the configuration for {}: {}",
					mod->ModuleFile, modex.what());
				if (user)
					user->WriteNotice(modname + ": " + modex.GetReason());
			}
		}

		// The description of this server may have changed - update it for WHOIS etc.
		ServerInstance->FakeClient->server->description = Config->ServerDesc;
		ServerInstance->Users.RehashServices();

		try
		{
			::Logs.CloseLogs();
			::Logs.OpenLogs(true);
		}
		catch (const CoreException& ex)
		{
			::Logs.Critical("LOG", "Cannot open log files: " + ex.GetReason());
			if (user)
				user->WriteNotice("Cannot open log files: " + ex.GetReason());
		}

		if (Config->RawLog && !old->RawLog)
		{
			for (auto* luser : ServerInstance->Users.GetLocalUsers())
				Log::NotifyRawIO(luser, MessageType::PRIVMSG);
		}

		Config = old;
	}
	else
	{
		// whoops, abort!
		ServerInstance->Config = old;
	}
}
