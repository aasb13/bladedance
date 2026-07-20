/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2020-2025 Sadie Powell <sadie@witchery.services>
 *   Copyright (C) 2013-2014 Attila Molnar <attilamolnar@hush.com>
 *   Copyright (C) 2012 Robby <robby@chatbelgie.be>
 *   Copyright (C) 2012 ChrisTX <xpipe@hotmail.de>
 *   Copyright (C) 2009 Daniel De Graaf <danieldg@inspircd.org>
 *   Copyright (C) 2008 Craig Edwards <brain@inspircd.org>
 *   Copyright (C) 2007-2008 Robin Burchell <robin+git@viroteck.net>
 *   Copyright (C) 2007 Dennis Friis <peavey@inspircd.org>
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

// Rust function declarations
extern "C" {
    struct StdString;
    StdString rust_generate_sid(const char* servername, size_t servername_length, const char* serverdesc, size_t serverdesc_length);
    void rust_uid_init(const char* sid, size_t sid_length);
    StdString rust_uid_get();
    StdString rust_server_get_public_name(const char* server_name, size_t server_name_length, const char* hide_server, size_t hide_server_length);
    void rust_server_send_metadata(const char* key, size_t key_length, const char* data, size_t data_length);
    void rust_server_send_metadata_ext(const void* ext, const char* key, size_t key_length, const char* data, size_t data_length);
}

void InspIRCd::HandleSignal(sig_atomic_t signal)
{
	switch (signal)
	{
		case SIGTERM:
			Exit(EXIT_FAILURE);

#ifndef _WIN32
		case SIGHUP:
			ServerInstance->SNO.WriteGlobalSno('r', "Rehashing due to SIGHUP");
			Rehash();
			break;
#endif

		default:
			::Logs.Debug("SIGNAL", "BUG: InspIRCd::SignalHandler: unknown signal: {}",
				signal);
			break;
	}
}

void InspIRCd::Exit(int status)
{
#ifdef _WIN32
	SetServiceStopped(status);
#endif
	this->Cleanup();
	ServerInstance = nullptr;
	delete this;
	exit(status);
}

void InspIRCd::Rehash(const std::string& uuid)
{
	if (!ConfigThread)
	{
		ConfigThread = new ConfigReaderThread(uuid);
		ConfigThread->Start();
	}
}

std::string UIDGenerator::GenerateSID(const std::string& servername, const std::string& serverdesc)
{
	return rust_generate_sid(servername.c_str(), servername.length(), serverdesc.c_str(), serverdesc.length()).data;
}

void UIDGenerator::init(const std::string& sid)
{
	rust_uid_init(sid.c_str(), sid.length());
}

/*
 * Retrieve the next valid UUID that is free for this server.
 */
std::string UIDGenerator::GetUID()
{
	while (true)
	{
		StdString rust_uid = rust_uid_get();
		std::string uid = std::string(rust_uid.data, rust_uid.length);

		if (!ServerInstance->Users.FindUUID(uid))
			return uid;

		// TODO: Make it use actual uuids instead of this crap
	}
}

const std::string& Server::GetPublicName() const
{
	// Use Rust implementation
	StdString result = rust_server_get_public_name(
		GetName().c_str(), GetName().length(),
		ServerInstance->Config->HideServer.c_str(), ServerInstance->Config->HideServer.length()
	);
	static std::string cached_result;
	cached_result.assign(result.data, result.length);
	return cached_result;
}

void Server::SendMetadata(const std::string& key, const std::string& data) const
{
	// Delegate to Rust implementation
	rust_server_send_metadata(key.c_str(), key.length(), data.c_str(), data.length());
}

void Server::SendMetadata(const Extensible* ext, const std::string& key, const std::string& data) const
{
	// Delegate to Rust implementation
	rust_server_send_metadata_ext(static_cast<const void*>(ext), key.c_str(), key.length(), data.c_str(), data.length());
}
