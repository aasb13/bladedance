/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2019, 2022-2023 Sadie Powell <sadie@witchery.services>
 *
 * Thin FFI for rust MatchCIDR port (cidr.rs): InspIRCd::Match and socket normalization.
 */

#include "inspircd.h"

extern "C" bool cidr_ffi_match_wildcard_ascii(const char* a, const char* b)
{
	return InspIRCd::Match(std::string(a), std::string(b), ascii_case_insensitive_map);
}

extern "C" bool cidr_ffi_match_normalized(const char* address_copy, const char* cidr_copy)
{
	irc::sockets::sockaddrs addr(false);
	if (!addr.from_ip(address_copy))
	{
		// The address could not be parsed.
		return false;
	}

	irc::sockets::cidr_mask mask(cidr_copy);
	irc::sockets::cidr_mask mask2(addr, mask.length);

	return mask == mask2;
}
