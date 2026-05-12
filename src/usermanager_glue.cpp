/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2019 iwalkalone <iwalkalone69@gmail.com>
 *   Copyright (C) 2013-2016, 2018 Attila Molnar <attilamolnar@hush.com>
 *   Copyright (C) 2013, 2018-2025 Sadie Powell <sadie@witchery.services>
 *   Copyright (C) 2013, 2015 Adam <Adam@anope.org>
 *   Copyright (C) 2012 Robby <robby@chatbelgie.be>
 *   Copyright (C) 2009-2010 Daniel De Graaf <danieldg@inspircd.org>
 *   Copyright (C) 2008-2009 Craig Edwards <brain@inspircd.org>
 *   Copyright (C) 2008 Robin Burchell <robin+git@viroteck.net>
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

extern "C" {
void rust_usermanager_add_user(UserManager* um, int socket, ListenSocket* via, const irc::sockets::sockaddrs* client,
	const irc::sockets::sockaddrs* server);
void rust_usermanager_quit_user(UserManager* um, User* user, const char* quitmessage, const char* operquitmessage);
void rust_usermanager_add_clone(UserManager* um, User* user);
void rust_usermanager_remove_clone_counts(UserManager* um, User* user);
void rust_usermanager_rehash_clone_counts(UserManager* um);
void rust_usermanager_rehash_services(UserManager* um);
void rust_usermanager_do_background_user_stuff(UserManager* um);
uint64_t rust_usermanager_next_already_sent_id(UserManager* um);
User* rust_usermanager_find(const UserManager* um, const char* nickuuid, bool fullyconnected);
User* rust_usermanager_find_nick(const UserManager* um, const char* nick, bool fullyconnected);
User* rust_usermanager_find_uuid(const UserManager* um, const char* uuid, bool fullyconnected);
}

UserManager::UserManager()
{
	// We need to define a constructor here to work around a Clang bug.
}

UserManager::~UserManager()
{
	for (const auto& [_, client] : clientlist)
		delete client;
}

void UserManager::AddUser(int socket, ListenSocket* via, const irc::sockets::sockaddrs& client, const irc::sockets::sockaddrs& server)
{
	rust_usermanager_add_user(this, socket, via, &client, &server);
}

void UserManager::QuitUser(User* user, const std::string& quitmessage, const std::string* operquitmessage)
{
	rust_usermanager_quit_user(this, user, quitmessage.c_str(), operquitmessage ? operquitmessage->c_str() : nullptr);
}

void UserManager::AddClone(User* user)
{
	rust_usermanager_add_clone(this, user);
}

void UserManager::RemoveCloneCounts(User* user)
{
	rust_usermanager_remove_clone_counts(this, user);
}

void UserManager::RehashCloneCounts()
{
	rust_usermanager_rehash_clone_counts(this);
}

void UserManager::RehashServices()
{
	rust_usermanager_rehash_services(this);
}

const UserManager::CloneCounts& UserManager::GetCloneCounts(User* user) const
{
	return UserManagerRustAccess::LookupCloneCounts(const_cast<UserManager*>(this), user);
}

void UserManager::DoBackgroundUserStuff()
{
	rust_usermanager_do_background_user_stuff(this);
}

uint64_t UserManager::NextAlreadySentId()
{
	return rust_usermanager_next_already_sent_id(this);
}

User* UserManager::Find(const std::string& nickuuid, bool fullyconnected)
{
	return rust_usermanager_find(this, nickuuid.c_str(), fullyconnected);
}

User* UserManager::FindNick(const std::string& nick, bool fullyconnected)
{
	return rust_usermanager_find_nick(this, nick.c_str(), fullyconnected);
}

User* UserManager::FindUUID(const std::string& uuid, bool fullyconnected)
{
	return rust_usermanager_find_uuid(this, uuid.c_str(), fullyconnected);
}
