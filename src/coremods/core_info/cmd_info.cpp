/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2020-2022, 2024-2025 Sadie Powell <sadie@witchery.services>
 *   Copyright (C) 2015 Robin Burchell <robin+git@viroteck.net>
 *   Copyright (C) 2013-2014, 2016 Attila Molnar <attilamolnar@hush.com>
 *   Copyright (C) 2012, 2019 Robby <robby@chatbelgie.be>
 *   Copyright (C) 2009-2010 Daniel De Graaf <danieldg@inspircd.org>
 *   Copyright (C) 2007 Craig Edwards <brain@inspircd.org>
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

enum
{
	// From RFC 1459
	RPL_INFO = 371,
	RPL_ENDOFINFO = 374,
};

CommandInfo::CommandInfo(Module* parent)
	: SplitCommand(parent, "INFO")
{
	penalty = 3000;
}

static const char* const lines[] = {
	"                   -/\\- \002Bladedance\002 -\\/-",
	" A modern Rust IRC server, forked from InspIRCd 4",
	" ",
	"\002Developer:\002 aasb13",
	" ",
	"\002Original project:\002 InspIRCd (GPLv2)",
	"\002InspIRCd maintainer:\002 Sadie Powell",
	nullptr
};

CmdResult CommandInfo::HandleLocal(LocalUser* user, const Params& parameters)
{
    if (GetUserLevel(user) > 0)
    {
        for (size_t idx = 0; lines[idx]; ++idx)
            user->WriteRemoteNumeric(RPL_INFO, lines[idx]);
    } else {
		user->WriteRemoteNumeric(RPL_INFO, "User level of above 0 is required to execute this command");
	}
    user->WriteRemoteNumeric(RPL_ENDOFINFO, "End of /INFO list");
    return CmdResult::SUCCESS;
}