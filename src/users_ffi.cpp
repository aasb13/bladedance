/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2019 linuxdaemon <linuxdaemon.irc@gmail.com>
 *   Copyright (C) 2013, 2017-2026 Sadie Powell <sadie@witchery.services>
 *
 * FFI for Rust users module (User base-class helpers).
 */

#include "inspircd.h"
#include "membership.h"

void UserRustAccess::InvalidateCache(User* u)
{
	u->cached_address.clear();
	u->cached_useraddress.clear();
	u->cached_userhost.clear();
	u->cached_realuserhost.clear();
	u->cached_mask.clear();
	u->cached_realmask.clear();
}

void UserRustAccess::SetCachedUserAddress(User* u, const uint8_t* data, size_t len)
{
	u->cached_useraddress.assign(reinterpret_cast<const char*>(data), len);
	u->cached_useraddress.shrink_to_fit();
}

void UserRustAccess::SetCachedUserHost(User* u, const uint8_t* data, size_t len)
{
	u->cached_userhost.assign(reinterpret_cast<const char*>(data), len);
	u->cached_userhost.shrink_to_fit();
}

void UserRustAccess::SetCachedRealUserHost(User* u, const uint8_t* data, size_t len)
{
	u->cached_realuserhost.assign(reinterpret_cast<const char*>(data), len);
	u->cached_realuserhost.shrink_to_fit();
}

void UserRustAccess::SetCachedMask(User* u, const uint8_t* data, size_t len)
{
	u->cached_mask.assign(reinterpret_cast<const char*>(data), len);
	u->cached_mask.shrink_to_fit();
}

void UserRustAccess::SetCachedRealMask(User* u, const uint8_t* data, size_t len)
{
	u->cached_realmask.assign(reinterpret_cast<const char*>(data), len);
	u->cached_realmask.shrink_to_fit();
}

void UserRustAccess::ReadRealUser(const User* u, const uint8_t** out, size_t* len)
{
	const auto& s = u->GetRealUser();
	*out = reinterpret_cast<const uint8_t*>(s.data());
	*len = s.size();
}

void UserRustAccess::ReadCachedAddress(User* u, const uint8_t** out, size_t* len)
{
	u->GetAddress();
	const auto& s = u->cached_address;
	*out = reinterpret_cast<const uint8_t*>(s.data());
	*len = s.size();
}

void UserRustAccess::ReadDisplayedUser(const User* u, const uint8_t** out, size_t* len)
{
	const auto& s = u->GetDisplayedUser();
	*out = reinterpret_cast<const uint8_t*>(s.data());
	*len = s.size();
}

void UserRustAccess::ReadDisplayedHost(const User* u, const uint8_t** out, size_t* len)
{
	const auto& s = u->GetDisplayedHost();
	*out = reinterpret_cast<const uint8_t*>(s.data());
	*len = s.size();
}

void UserRustAccess::ReadRealHost(const User* u, const uint8_t** out, size_t* len)
{
	const auto& s = u->GetRealHost();
	*out = reinterpret_cast<const uint8_t*>(s.data());
	*len = s.size();
}

void UserRustAccess::ReadNick(const User* u, const uint8_t** out, size_t* len)
{
	const auto& s = u->nick;
	*out = reinterpret_cast<const uint8_t*>(s.data());
	*len = s.size();
}

bool UserRustAccess::ModeIdIsSet(const User* u, unsigned int id)
{
	if (id >= ModeParser::MODEID_MAX)
		return false;
	return u->modes[id];
}

bool UserRustAccess::NoticeMaskBit(const User* u, unsigned char sm)
{
	const unsigned idx = static_cast<unsigned>(sm) - 65U;
	if (idx >= u->snomasks.size())
		return false;
	return u->snomasks[idx];
}

bool UserRustAccess::SharesChannelWith(const User* u, User* other)
{
	for (const auto* memb : u->chans)
	{
		if (memb->chan->HasUser(other))
			return true;
	}
	return false;
}

INSP_RUST_FFI_IMPL_BEGIN
extern "C" {

void user_ffi_invalidate_cache(User* u)
{
	UserRustAccess::InvalidateCache(u);
}

void user_ffi_user_set_cached_useraddress(User* u, const uint8_t* data, size_t len)
{
	UserRustAccess::SetCachedUserAddress(u, data, len);
}

void user_ffi_user_set_cached_userhost(User* u, const uint8_t* data, size_t len)
{
	UserRustAccess::SetCachedUserHost(u, data, len);
}

void user_ffi_user_set_cached_realuserhost(User* u, const uint8_t* data, size_t len)
{
	UserRustAccess::SetCachedRealUserHost(u, data, len);
}

void user_ffi_user_set_cached_mask(User* u, const uint8_t* data, size_t len)
{
	UserRustAccess::SetCachedMask(u, data, len);
}

void user_ffi_user_set_cached_realmask(User* u, const uint8_t* data, size_t len)
{
	UserRustAccess::SetCachedRealMask(u, data, len);
}

void user_ffi_user_read_real_user(const User* u, const uint8_t** out, size_t* len)
{
	UserRustAccess::ReadRealUser(u, out, len);
}

void user_ffi_user_read_cached_address(User* u, const uint8_t** out, size_t* len)
{
	UserRustAccess::ReadCachedAddress(u, out, len);
}

void user_ffi_user_read_displayed_user(const User* u, const uint8_t** out, size_t* len)
{
	UserRustAccess::ReadDisplayedUser(u, out, len);
}

void user_ffi_user_read_displayed_host(const User* u, const uint8_t** out, size_t* len)
{
	UserRustAccess::ReadDisplayedHost(u, out, len);
}

void user_ffi_user_read_real_host(const User* u, const uint8_t** out, size_t* len)
{
	UserRustAccess::ReadRealHost(u, out, len);
}

void user_ffi_user_read_nick(const User* u, const uint8_t** out, size_t* len)
{
	UserRustAccess::ReadNick(u, out, len);
}

void* user_ffi_find_user_mode_char(unsigned char m)
{
	return ServerInstance->Modes.FindMode(static_cast<char>(m), MODETYPE_USER);
}

bool user_ffi_user_mode_id_is_set(const User* u, unsigned int id)
{
	return UserRustAccess::ModeIdIsSet(u, id);
}

size_t user_ffi_usermode_handlers_fill(void** out, size_t max_out)
{
	size_t i = 0;
	for (const auto& pair : ServerInstance->Modes.GetModes(MODETYPE_USER))
	{
		if (i >= max_out)
			break;
		out[i++] = pair.second;
	}
	return i;
}

unsigned int user_ffi_modehandler_id(void* mh)
{
	auto* m = static_cast<ModeHandler*>(mh);
	return m ? m->GetId() : ModeParser::MODEID_MAX;
}

char user_ffi_modehandler_char(void* mh)
{
	auto* m = static_cast<ModeHandler*>(mh);
	return m ? m->GetModeChar() : 0;
}

bool user_ffi_modehandler_needs_param_on_list(void* mh)
{
	auto* m = static_cast<ModeHandler*>(mh);
	return m && m->NeedsParam(true);
}

size_t user_ffi_modehandler_user_parameter_copy(User* u, void* mh, uint8_t* buf, size_t cap)
{
	auto* modehandler = static_cast<ModeHandler*>(mh);
	if (!modehandler)
		return 0;
	const std::string val = modehandler->GetUserParameter(u);
	if (val.empty() || val.size() >= cap)
		return 0;
	memcpy(buf, val.data(), val.size());
	return val.size();
}

bool user_ffi_user_notice_mask_bit(const User* u, unsigned char sm)
{
	return UserRustAccess::NoticeMaskBit(u, sm);
}

bool user_ffi_user_shares_channel_with(const User* u, User* other)
{
	return UserRustAccess::SharesChannelWith(u, other);
}

} // extern "C"
INSP_RUST_FFI_IMPL_END
