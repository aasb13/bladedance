/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2019, 2022-2023 Sadie Powell <sadie@witchery.services>
 */

#include "inspircd.h"

extern "C" bool rust_match_cidr(const char* address, const char* cidr_mask, bool match_with_username);

bool irc::sockets::MatchCIDR(const std::string& address, const std::string& cidr_mask, bool match_with_username)
{
	return rust_match_cidr(address.c_str(), cidr_mask.c_str(), match_with_username);
}