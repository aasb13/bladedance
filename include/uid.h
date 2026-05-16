/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2013, 2019, 2021-2022 Sadie Powell <sadie@witchery.services>
 *   Copyright (C) 2013 Attila Molnar <attilamolnar@hush.com>
 *   Copyright (C) 2012 Robby <robby@chatbelgie.be>
 *   Copyright (C) 2007 Robin Burchell <robin+git@viroteck.net>
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

#pragma once

class CoreExport UIDGenerator final
{
public:
	/** Initializes this UID generator with the given SID
	 * @param sid SID that conforms to InspIRCd::IsSID()
	 */
	void init(const std::string& sid);

	/** Returns the next available UID for this server.
	 */
	std::string GetUID();

	/** Generates a pseudorandom SID based on a servername and a description
	 * Guaranteed to return the same if invoked with the same parameters
	 * @param servername The server name to use as seed
	 * @param serverdesc The server description to use as seed
	 * @return A valid SID
	 */
	static std::string GenerateSID(const std::string& servername, const std::string& serverdesc);
};
