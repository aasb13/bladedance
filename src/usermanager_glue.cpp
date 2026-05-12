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
#include "clientprotocolmsg.h"
#include "iohook.h"
#include "timeutils.h"
#include "xline.h"

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

// --- UserManagerRustAccess (private fields only; no control flow) -----------

const UserManager::CloneCounts& UserManagerRustAccess::LookupCloneCounts(UserManager* um, User* user)
{
	UserManager::CloneMap::const_iterator it = um->clonemap.find(user->GetCIDRMask());
	if (it != um->clonemap.end())
		return it->second;
	else
		return um->zeroclonecounts;
}

void UserManagerRustAccess::IncUnknown(UserManager* um)
{
	um->unknown_count++;
}

void UserManagerRustAccess::DecUnknown(UserManager* um)
{
	um->unknown_count--;
}

void UserManagerRustAccess::ClientListInsert(UserManager* um, LocalUser* lu)
{
	um->clientlist[lu->nick] = lu;
}

void UserManagerRustAccess::LocalUsersPushFront(UserManager* um, LocalUser* lu)
{
	um->local_users.push_front(lu);
}

void UserManagerRustAccess::LocalUsersErase(UserManager* um, LocalUser* lu)
{
	um->local_users.erase(lu);
}

void UserManagerRustAccess::CloneMapClear(UserManager* um)
{
	um->clonemap.clear();
}

void UserManagerRustAccess::CloneMapAddEntry(UserManager* um, User* user)
{
	UserManager::CloneCounts& counts = um->clonemap[user->GetCIDRMask()];
	counts.global++;
	if (IS_LOCAL(user))
		counts.local++;
}

void UserManagerRustAccess::CloneMapRemoveEntry(UserManager* um, User* user)
{
	UserManager::CloneMap::iterator it = um->clonemap.find(user->GetCIDRMask());
	if (it != um->clonemap.end())
	{
		UserManager::CloneCounts& counts = it->second;
		counts.global--;
		if (counts.global == 0)
		{
			// No more users from this IP, remove entry from the map
			um->clonemap.erase(it);
			return;
		}

		if (IS_LOCAL(user))
			counts.local--;
	}
}

bool UserManagerRustAccess::ClientListEraseNick(UserManager* um, const std::string& nick)
{
	return um->clientlist.erase(nick);
}

void UserManagerRustAccess::UuidListErase(UserManager* um, const std::string& uuid)
{
	um->uuidlist.erase(uuid);
}

uint64_t UserManagerRustAccess::GetAlreadySentId(UserManager* um)
{
	return um->already_sent_id;
}

void UserManagerRustAccess::SetAlreadySentId(UserManager* um, uint64_t v)
{
	um->already_sent_id = v;
}

size_t UserManagerRustAccess::LocalUsersSize(UserManager* um)
{
	return um->local_users.size();
}

struct UserManagerRustAccess::ClientIter
{
	UserMap::iterator it;
	UserMap::iterator end;
};

UserManagerRustAccess::ClientIter* UserManagerRustAccess::ClientIterNew(UserManager* um)
{
	return new ClientIter{ um->clientlist.begin(), um->clientlist.end() };
}

User* UserManagerRustAccess::ClientIterNext(ClientIter* it)
{
	if (!it || it->it == it->end)
		return nullptr;
	User* u = it->it->second;
	++it->it;
	return u;
}

void UserManagerRustAccess::ClientIterFree(ClientIter* it)
{
	delete it;
}

struct UserManagerRustAccess::LocalIter
{
	UserManager::LocalList::iterator it;
	UserManager::LocalList::iterator end;
};

UserManagerRustAccess::LocalIter* UserManagerRustAccess::LocalIterNew(UserManager* um)
{
	return new LocalIter{ um->local_users.begin(), um->local_users.end() };
}

LocalUser* UserManagerRustAccess::LocalIterNext(LocalIter* it)
{
	if (!it || it->it == it->end)
		return nullptr;
	LocalUser* u = *it->it;
	++it->it;
	return u;
}

void UserManagerRustAccess::LocalIterFree(LocalIter* it)
{
	delete it;
}

void UserManagerRustAccess::ServicesSwapFromVector(UserManager* um, User* const* users, size_t count)
{
	UserManager::ServiceList newservices;
	newservices.reserve(count);
	for (size_t i = 0; i < count; ++i)
		newservices.push_back(users[i]);
	std::swap(um->all_services, newservices);
}

// --- Protocol object that cannot be expressed in Rust -----------------------

namespace
{
	class WriteCommonQuit final
		: public User::ForEachNeighborHandler
	{
		ClientProtocol::Messages::Quit quitmsg;
		ClientProtocol::Event quitevent;
		ClientProtocol::Messages::Quit operquitmsg;
		ClientProtocol::Event operquitevent;

		void Execute(LocalUser* user) override
		{
			user->Send(user->IsOper() ? operquitevent : quitevent);
		}

	public:
		WriteCommonQuit(User* user, const std::string& msg, const std::string& opermsg)
			: quitmsg(user, msg)
			, quitevent(ServerInstance->GetRFCEvents().quit, quitmsg)
			, operquitmsg(user, opermsg)
			, operquitevent(ServerInstance->GetRFCEvents().quit, operquitmsg)
		{
			user->ForEachNeighbor(*this, false);
		}
	};
} // namespace

static thread_local std::string tls_quitmsg;
static thread_local std::string tls_operquitmsg;
static thread_local std::string tls_scratch;

INSP_RUST_FFI_IMPL_BEGIN
extern "C" {

void um_ffi_write_common_quit(User* user, const char* quitmsg, const char* operquitmsg)
{
	WriteCommonQuit w(user, quitmsg, operquitmsg);
}

time_t um_ffi_server_time()
{
	return ServerInstance->Time();
}

time_t um_ffi_local_user_nextping(LocalUser* lu)
{
	return lu->nextping;
}

void um_ffi_local_user_set_nextping(LocalUser* lu, time_t t)
{
	lu->nextping = t;
}

unsigned int um_ffi_local_user_lastping(LocalUser* lu)
{
	return lu->lastping;
}

void um_ffi_local_user_set_lastping(LocalUser* lu, unsigned int v)
{
	lu->lastping = v ? 1U : 0U;
}

unsigned long um_ffi_local_user_class_pingtime(LocalUser* lu)
{
	return lu->GetClass()->pingtime;
}

const char* um_ffi_duration_to_long_string(unsigned long secs)
{
	tls_scratch = Duration::ToLongString(secs);
	return tls_scratch.c_str();
}

void* um_ffi_streamsocket_get_iohook(UserIOHandler* eh)
{
	return static_cast<void*>(eh->GetIOHook());
}

bool um_ffi_iohook_ping(void* hook)
{
	return hook && static_cast<IOHook*>(hook)->Ping();
}

void* um_ffi_iohook_next_in_chain(void* hook)
{
	if (!hook)
		return nullptr;
	IOHookMiddle* middlehook = IOHookMiddle::ToMiddleHook(static_cast<IOHook*>(hook));
	return middlehook ? static_cast<void*>(middlehook->GetNextHook()) : nullptr;
}

void um_ffi_local_user_send_irc_ping(LocalUser* lu)
{
	ClientProtocol::Messages::Ping ping;
	lu->Send(ServerInstance->GetRFCEvents().ping, ping);
}

int64_t um_ffi_local_user_connection_timeout_deadline(LocalUser* lu)
{
	if (!lu->GetClass())
		return -1;
	return static_cast<int64_t>(static_cast<time_t>(lu->signon + lu->GetClass()->connection_timeout));
}

bool um_ffi_mod_on_check_ready_is_passthru(LocalUser* lu)
{
	ModResult res;
	FIRST_MOD_RESULT(OnCheckReady, res, (lu));
	return res == MOD_RES_PASSTHRU;
}

void um_ffi_local_user_full_connect(LocalUser* lu)
{
	lu->FullConnect();
}

size_t um_ffi_listen_socket_iohookprov_count(ListenSocket* via)
{
	return via->iohookprovs.size();
}

bool um_ffi_listen_socket_iohookprov_empty_name(ListenSocket* via, size_t idx)
{
	return via->iohookprovs[idx].GetProvider().empty();
}

bool um_ffi_listen_socket_iohookprov_valid(ListenSocket* via, size_t idx)
{
	return static_cast<bool>(via->iohookprovs[idx]);
}

void um_ffi_log_listen_iohook_nonexistent(ListenSocket* via, size_t idx)
{
	ListenSocket::IOHookProvRef& ref = via->iohookprovs[idx];
	const char* hooktype = idx == 0 ? "hook" : "sslprofile";
	ServerInstance->Logs.Warning("USERS", "Non-existent I/O hook '{}' in <bind:{}> tag at {}",
		ref.GetProvider(), hooktype, via->bind_tag->source.str());
}

const char* um_ffi_insp_format_misconfigured_iohook(size_t idx)
{
	const char* hooktype = idx == 0 ? "hook" : "sslprofile";
	tls_scratch = INSP_FORMAT("Internal error handling connection (misconfigured {})", hooktype);
	return tls_scratch.c_str();
}

void um_ffi_listen_socket_iohookprov_on_accept(ListenSocket* via, size_t idx, UserIOHandler* eh,
	const irc::sockets::sockaddrs* client, const irc::sockets::sockaddrs* server)
{
	via->iohookprovs[idx]->OnAccept(eh, *client, *server);
}

bool um_ffi_useriohandler_error_nonempty(UserIOHandler* eh)
{
	return !eh->GetError().empty();
}

const char* um_ffi_useriohandler_error_cstr(UserIOHandler* eh)
{
	tls_scratch = eh->GetError();
	return tls_scratch.c_str();
}

LocalUser* um_ffi_local_user_new(int socket, const irc::sockets::sockaddrs* client, const irc::sockets::sockaddrs* server)
{
	return new LocalUser(socket, *client, *server);
}

void um_ffi_log_users_new_fd(int socket)
{
	ServerInstance->Logs.Debug("USERS", "New user fd: {}", socket);
}

UserIOHandler* um_ffi_local_user_iohandler(LocalUser* lu)
{
	return &lu->eh;
}

void um_ffi_foreach_mod_on_user_init(LocalUser* lu)
{
	FOREACH_MOD(OnUserInit, (lu));
}

bool um_ffi_socket_engine_add_fd(UserIOHandler* eh, int event_mask)
{
	return SocketEngine::AddFd(eh, event_mask);
}

void um_ffi_log_users_internal_error()
{
	ServerInstance->Logs.Debug("USERS", "Internal error on new connection");
}

void um_ffi_log_softlimit_warning()
{
	ServerInstance->SNO.WriteToSnoMask('a', "Warning: softlimit value has been reached: {} clients", ServerInstance->Config->SoftLimit);
}

unsigned long um_ffi_config_soft_limit()
{
	return ServerInstance->Config->SoftLimit;
}

bool um_ffi_local_user_find_connect_class(LocalUser* lu)
{
	return lu->FindConnectClass();
}

void um_ffi_local_user_set_exempt_from_eline(LocalUser* lu, bool exempt)
{
	lu->exempt = exempt;
}

XLine* um_ffi_xline_matches_E(LocalUser* lu)
{
	return ServerInstance->XLines->MatchesLine("E", lu);
}

BanCacheHit* um_ffi_ban_cache_get_hit(const char* addr)
{
	return ServerInstance->BanCache.GetHit(addr);
}

bool um_ffi_bancache_hit_type_non_empty(BanCacheHit* b)
{
	return b && !b->Type.empty();
}

const char* um_ffi_bancache_hit_reason_cstr(BanCacheHit* b)
{
	static thread_local std::string s;
	if (!b)
	{
		s.clear();
		return s.c_str();
	}
	s = b->Reason.to_std_string();
	return s.c_str();
}

void um_ffi_log_bancache_positive(const char* addr)
{
	ServerInstance->Logs.Debug("BANCACHE", "Positive hit for {}", addr);
}

void um_ffi_log_bancache_negative(const char* addr)
{
	ServerInstance->Logs.Debug("BANCACHE", "Negative hit for {}", addr);
}

bool um_ffi_config_xline_message_empty()
{
	return ServerInstance->Config->XLineMessage.empty();
}

void um_ffi_local_user_write_numeric_banned(LocalUser* lu)
{
	lu->WriteNumeric(ERR_YOUREBANNEDCREEP, ServerInstance->Config->XLineMessage);
}

const char* um_ffi_local_user_get_address_cstr(LocalUser* lu)
{
	static thread_local std::string s;
	s = lu->GetAddress();
	return s.c_str();
}

XLine* um_ffi_xline_matches_Z(LocalUser* lu)
{
	return ServerInstance->XLines->MatchesLine("Z", lu);
}

void um_ffi_xline_apply(XLine* line, LocalUser* lu)
{
	line->Apply(lu);
}

bool um_ffi_config_raw_log()
{
	return ServerInstance->Config->RawLog;
}

void um_ffi_log_notify_raw_io(LocalUser* lu)
{
	Log::NotifyRawIO(lu, MessageType::NOTICE);
}

void um_ffi_foreach_mod_on_change_remote_address(LocalUser* lu)
{
	FOREACH_MOD(OnChangeRemoteAddress, (lu));
}

void um_ffi_foreach_mod_on_user_post_init(LocalUser* lu)
{
	FOREACH_MOD(OnUserPostInit, (lu));
}

bool um_ffi_user_is_server(User* u)
{
	return IS_SERVER(u);
}

LocalUser* um_ffi_user_as_local(User* u)
{
	return IS_LOCAL(u);
}

void um_ffi_log_users_bug_quitting(const char* nick)
{
	ServerInstance->Logs.Debug("USERS", "BUG: Tried to quit quitting user: {}", nick);
}

void um_ffi_log_users_bug_server(const char* nick)
{
	ServerInstance->Logs.Debug("USERS", "BUG: Tried to quit server user: {}", nick);
}

bool um_ffi_quit_user_run_prequit(LocalUser* lu, const char* quitmessage, const char* operquitmessage_or_null)
{
	tls_quitmsg = quitmessage;
	tls_operquitmsg.clear();
	if (operquitmessage_or_null)
		tls_operquitmsg.assign(operquitmessage_or_null);

	if (lu)
	{
		ModResult modres;
		FIRST_MOD_RESULT(OnUserPreQuit, modres, (lu, tls_quitmsg, tls_operquitmsg));
		if (modres == MOD_RES_DENY)
			return true;
	}

	if (tls_quitmsg.length() > ServerInstance->Config->Limits.MaxQuit)
		tls_quitmsg.erase(ServerInstance->Config->Limits.MaxQuit + 1);

	if (tls_operquitmsg.empty())
		tls_operquitmsg.assign(tls_quitmsg);
	else if (tls_operquitmsg.length() > ServerInstance->Config->Limits.MaxQuit)
		tls_operquitmsg.erase(ServerInstance->Config->Limits.MaxQuit + 1);

	return false;
}

const char* um_ffi_quit_user_tls_quit()
{
	return tls_quitmsg.c_str();
}

const char* um_ffi_quit_user_tls_oper()
{
	return tls_operquitmsg.c_str();
}

void um_ffi_user_set_quitting(User* u)
{
	u->quitting = true;
}

void um_ffi_log_quit_user(const char* uuid, const char* nick, const char* quitmessage)
{
	ServerInstance->Logs.Debug("USERS", "QuitUser: {}={} '{}'", uuid, nick, quitmessage);
}

void um_ffi_local_user_send_error_quit(LocalUser* lu, const char* operquitmsg)
{
	ClientProtocol::Messages::Error errormsg(INSP_FORMAT("Closing link: ({}) [{}]", lu->GetRealUserHost(), operquitmsg));
	lu->Send(ServerInstance->GetRFCEvents().error, errormsg);
}

void um_ffi_global_culls_add_item(User* u)
{
	// GlobalCulls was removed - function now does nothing
}

void um_ffi_foreach_mod_on_user_quit(User* user, const char* quitmsg, const char* operquitmsg)
{
	std::string q(quitmsg);
	std::string o(operquitmsg);
	FOREACH_MOD(OnUserQuit, (user, q, o));
}

void um_ffi_foreach_mod_on_user_disconnect(LocalUser* lu)
{
	FOREACH_MOD(OnUserDisconnect, (lu));
}

void um_ffi_local_user_eh_close(LocalUser* lu)
{
	lu->eh.Close();
}

void um_ffi_sno_write_client_exiting(const char* realmask, const char* addr, const char* operquitmsg)
{
	ServerInstance->SNO.WriteToSnoMask('q', "Client exiting: {} ({}) [{}]", realmask, addr, operquitmsg);
}

const char* um_ffi_user_get_real_mask_cstr(User* u)
{
	static thread_local std::string s;
	s = u->GetRealMask();
	return s.c_str();
}

const char* um_ffi_user_get_address_cstr(User* u)
{
	static thread_local std::string s;
	s = u->GetAddress();
	return s.c_str();
}

void um_ffi_log_users_bug_nick_not_found(const char* nick)
{
	ServerInstance->Logs.Debug("USERS", "BUG: Nick not found in clientlist, cannot remove: {}", nick);
}

void um_ffi_user_purge_empty_channels(User* u)
{
	u->PurgeEmptyChannels();
}

void um_ffi_user_oper_logout(User* u)
{
	u->OperLogout();
}

bool um_ffi_user_quitting(User* u)
{
	return u->quitting;
}

User* um_ffi_user_manager_find_nick_impl(UserManager* um, const char* nick, bool fullyconnected)
{
	if (!nick || !nick[0])
		return nullptr;

	UserMap::iterator uiter = um->clientlist.find(nick);
	if (uiter == um->clientlist.end())
		return nullptr;

	User* user = uiter->second;
	if (fullyconnected && !user->IsFullyConnected())
		return nullptr;

	return user;
}

User* um_ffi_user_manager_find_uuid_impl(UserManager* um, const char* uuid, bool fullyconnected)
{
	if (!uuid || !uuid[0])
		return nullptr;

	UserMap::iterator uiter = um->uuidlist.find(uuid);
	if (uiter == um->uuidlist.end())
		return nullptr;

	User* user = uiter->second;
	if (fullyconnected && !user->IsFullyConnected())
		return nullptr;

	return user;
}

const char* um_ffi_user_get_nick_cstr(User* u)
{
	return u->nick.c_str();
}

const char* um_ffi_user_get_uuid_cstr(User* u)
{
	return u->uuid.c_str();
}

bool um_ffi_user_is_fully_connected(User* u)
{
	return u->IsFullyConnected();
}

bool um_ffi_user_server_is_service(User* u)
{
	return u->server->IsService();
}

unsigned long um_ffi_local_user_command_flood_penalty(LocalUser* lu)
{
	return lu->CommandFloodPenalty;
}

size_t um_ffi_local_user_eh_get_sendq_size(LocalUser* lu)
{
	return lu->eh.GetSendQSize();
}

unsigned long um_ffi_local_user_get_class_commandrate(LocalUser* lu)
{
	return lu->GetClass()->commandrate;
}

void um_ffi_local_user_set_command_flood_penalty(LocalUser* lu, unsigned long v)
{
	lu->CommandFloodPenalty = v;
}

void um_ffi_local_user_eh_on_data_ready(LocalUser* lu)
{
	lu->eh.OnDataReady();
}

unsigned int um_ffi_local_user_connected(LocalUser* lu)
{
	return lu->connected;
}

void um_ffi_local_user_set_already_sent(LocalUser* lu, uint64_t v)
{
	lu->already_sent = v;
}

void um_ffi_user_manager_rust_access_inc_unknown(UserManager* um)
{
	UserManagerRustAccess::IncUnknown(um);
}

void um_ffi_user_manager_rust_access_client_insert(UserManager* um, LocalUser* lu)
{
	UserManagerRustAccess::ClientListInsert(um, lu);
}

void um_ffi_user_manager_rust_access_local_push_front(UserManager* um, LocalUser* lu)
{
	UserManagerRustAccess::LocalUsersPushFront(um, lu);
}

void um_ffi_user_manager_rust_access_local_erase(UserManager* um, LocalUser* lu)
{
	UserManagerRustAccess::LocalUsersErase(um, lu);
}

void um_ffi_user_manager_rust_access_clonemap_clear(UserManager* um)
{
	UserManagerRustAccess::CloneMapClear(um);
}

void um_ffi_user_manager_rust_access_clonemap_add(UserManager* um, User* u)
{
	UserManagerRustAccess::CloneMapAddEntry(um, u);
}

void um_ffi_user_manager_rust_access_clonemap_remove(UserManager* um, User* u)
{
	UserManagerRustAccess::CloneMapRemoveEntry(um, u);
}

bool um_ffi_user_manager_rust_access_client_erase_nick_cstr(UserManager* um, const char* nick)
{
	return UserManagerRustAccess::ClientListEraseNick(um, nick);
}

void um_ffi_user_manager_rust_access_uuid_erase_cstr(UserManager* um, const char* uuid)
{
	UserManagerRustAccess::UuidListErase(um, uuid);
}

uint64_t um_ffi_user_manager_rust_access_get_already_sent_id(UserManager* um)
{
	return UserManagerRustAccess::GetAlreadySentId(um);
}

void um_ffi_user_manager_rust_access_set_already_sent_id(UserManager* um, uint64_t v)
{
	UserManagerRustAccess::SetAlreadySentId(um, v);
}

void* um_ffi_user_manager_rust_access_client_iter_new(UserManager* um)
{
	return UserManagerRustAccess::ClientIterNew(um);
}

User* um_ffi_user_manager_rust_access_client_iter_next(void* it)
{
	return UserManagerRustAccess::ClientIterNext(static_cast<UserManagerRustAccess::ClientIter*>(it));
}

void um_ffi_user_manager_rust_access_client_iter_free(void* it)
{
	UserManagerRustAccess::ClientIterFree(static_cast<UserManagerRustAccess::ClientIter*>(it));
}

void* um_ffi_user_manager_rust_access_local_iter_new(UserManager* um)
{
	return UserManagerRustAccess::LocalIterNew(um);
}

LocalUser* um_ffi_user_manager_rust_access_local_iter_next(void* it)
{
	return UserManagerRustAccess::LocalIterNext(static_cast<UserManagerRustAccess::LocalIter*>(it));
}

void um_ffi_user_manager_rust_access_local_iter_free(void* it)
{
	UserManagerRustAccess::LocalIterFree(static_cast<UserManagerRustAccess::LocalIter*>(it));
}

void um_ffi_local_user_connect_class_dec_use_count(LocalUser* lu)
{
	if (lu->GetClass())
		lu->GetClass()->use_count--;
}

void um_ffi_user_manager_rust_access_services_swap(UserManager* um, User* const* users, size_t count)
{
	UserManagerRustAccess::ServicesSwapFromVector(um, users, count);
}

void um_ffi_user_manager_rust_access_dec_unknown(UserManager* um)
{
	UserManagerRustAccess::DecUnknown(um);
}

size_t um_ffi_user_manager_rust_access_local_users_size(UserManager* um)
{
	return UserManagerRustAccess::LocalUsersSize(um);
}

} // extern "C"
INSP_RUST_FFI_IMPL_END
