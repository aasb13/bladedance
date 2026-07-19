/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2013, 2017, 2021-2022 Sadie Powell <sadie@witchery.services>
 *   Copyright (C) 2012 Robby <robby@chatbelgie.be>
 *   Copyright (C) 2009 Uli Schlachter <psychon@znc.in>
 *   Copyright (C) 2008 Craig Edwards <brain@inspircd.org>
 *   Copyright (C) 2007-2008 Robin Burchell <robin+git@viroteck.net>
 *   Copyright (C) 2007-2008 Dennis Friis <peavey@inspircd.org>
 */

#include "inspircd.h"

extern "C" bool rust_wildcard_match(const unsigned char* str, const unsigned char* wild, const unsigned char* map);
extern "C" bool rust_match_cidr(const char* address, const char* cidr_mask, bool match_with_username);
extern "C" bool rust_inspircd_match_mask(const char* masks, const char* hostname, const char* ipaddr, const unsigned char* ascii_map);

__attribute__ ((visibility ("default"))) bool Match(const std::string& str, const std::string& mask, const unsigned char* map)
{
	if (!map)
		map = national_case_insensitive_map;

	return rust_wildcard_match(
		reinterpret_cast<const unsigned char*>(str.c_str()),
		reinterpret_cast<const unsigned char*>(mask.c_str()),
		map);
}

__attribute__ ((visibility ("default"))) bool Match(const char* str, const char* mask, const unsigned char* map)
{
	if (!map)
		map = national_case_insensitive_map;

	return rust_wildcard_match(
		reinterpret_cast<const unsigned char*>(str),
		reinterpret_cast<const unsigned char*>(mask),
		map);
}

__attribute__ ((visibility ("default"))) bool MatchCIDR(const std::string& str, const std::string& mask, const unsigned char* map)
{
	if (!map)
		map = national_case_insensitive_map;

	if (rust_match_cidr(str.c_str(), mask.c_str(), true))
		return true;

	return rust_wildcard_match(
		reinterpret_cast<const unsigned char*>(str.c_str()),
		reinterpret_cast<const unsigned char*>(mask.c_str()),
		map);
}

__attribute__ ((visibility ("default"))) bool MatchCIDR(const char* str, const char* mask, const unsigned char* map)
{
	if (!map)
		map = national_case_insensitive_map;

	if (rust_match_cidr(str, mask, true))
		return true;

	return rust_wildcard_match(
		reinterpret_cast<const unsigned char*>(str),
		reinterpret_cast<const unsigned char*>(mask),
		map);
}

__attribute__ ((visibility ("default"))) bool MatchMask(const std::string& masks, const std::string& hostname, const std::string& address)
{
	return rust_inspircd_match_mask(masks.c_str(), hostname.c_str(), address.c_str(), ascii_case_insensitive_map);
}
