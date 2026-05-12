/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2018, 2020-2022 Sadie Powell <sadie@witchery.services>
 *   Copyright (C) 2014, 2016 Attila Molnar <attilamolnar@hush.com>
 *   Copyright (C) 2012 Robby <robby@chatbelgie.be>
 *   Copyright (C) 2009 Uli Schlachter <psychon@znc.in>
 *   Copyright (C) 2009 Daniel De Graaf <danieldg@inspircd.org>
 *   Copyright (C) 2006 Craig Edwards <brain@inspircd.org>
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
#include "core_info.h"

// Rust function declarations
extern "C" {
    struct StdString;
    void* CommandAdmin_Create();
    void CommandAdmin_Destroy(void* ptr);
    void CommandAdmin_SetAdminName(void* ptr, const char* name, size_t name_length);
    void CommandAdmin_SetAdminDescription(void* ptr, const char* desc, size_t desc_length);
    void CommandAdmin_SetAdminEmail(void* ptr, const char* email, size_t email_length);
    StdString CommandAdmin_HandleAdmin(void* ptr, int user_level, const char* server_name, size_t server_name_length);
}

CommandAdmin::CommandAdmin(Module* parent)
	: ServerTargetCommand(parent, "ADMIN")
	, rust_instance(nullptr)
{
	penalty = 2000;
	syntax = { "[<servername>]" };
	
	// Create Rust instance
	rust_instance = CommandAdmin_Create();
}

CommandAdmin::~CommandAdmin()
{
	// Destroy Rust instance
	if (rust_instance)
		CommandAdmin_Destroy(rust_instance);
}

CmdResult CommandAdmin::Handle(User* user, const Params& parameters)
{
	if (!rust_instance)
		return CmdResult::FAILURE;

	// Call Rust implementation
	StdString result = CommandAdmin_HandleAdmin(
		rust_instance,
		GetUserLevel(user),
		ServerInstance->Config->GetServerName().c_str(),
		ServerInstance->Config->GetServerName().length()
	);

	if (result.data && result.length > 0)
	{
		// Split the response by newlines and send each line as numeric
		std::string response(result.data, result.length);
		size_t pos = 0;
		size_t newline_pos;
		
		while ((newline_pos = response.find('\n', pos)) != std::string::npos) {
			std::string line = response.substr(pos, newline_pos - pos);
			
			// Parse numeric and message from line like "256 servername :message"
			size_t space_pos = line.find(' ');
			if (space_pos != std::string::npos) {
				size_t msg_pos = line.find(" :", space_pos);
				if (msg_pos != std::string::npos) {
					std::string numeric_str = line.substr(0, space_pos);
					std::string server_name = line.substr(space_pos + 1, msg_pos - space_pos - 1);
					std::string message = line.substr(msg_pos + 2);
					
					unsigned int numeric = std::stoul(numeric_str);
					user->WriteRemoteNumeric(numeric, server_name, message);
				}
			}
			pos = newline_pos + 1;
		}
		
		// Send the last line if there's no trailing newline
		if (pos < response.length()) {
			std::string line = response.substr(pos);
			size_t space_pos = line.find(' ');
			if (space_pos != std::string::npos) {
				size_t msg_pos = line.find(" :", space_pos);
				if (msg_pos != std::string::npos) {
					std::string numeric_str = line.substr(0, space_pos);
					std::string server_name = line.substr(space_pos + 1, msg_pos - space_pos - 1);
					std::string message = line.substr(msg_pos + 2);
					
					unsigned int numeric = std::stoul(numeric_str);
					user->WriteRemoteNumeric(numeric, server_name, message);
				}
			}
		}
	}
	
	return CmdResult::SUCCESS;
}
