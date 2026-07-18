/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2019 Matt Schatz <genius3000@g3k.solutions>
 *   Copyright (C) 2018 linuxdaemon <linuxdaemon.irc@gmail.com>
 *   Copyright (C) 2012-2013 Attila Molnar <attilamolnar@hush.com>
 *   Copyright (C) 2012, 2014, 2017-2018, 2020-2026 Sadie Powell <sadie@witchery.services>
 *   Copyright (C) 2012 Robby <robby@chatbelgie.be>
 *   Copyright (C) 2012 ChrisTX <xpipe@hotmail.de>
 *   Copyright (C) 2009-2010 Daniel De Graaf <danieldg@inspircd.org>
 *   Copyright (C) 2007 Dennis Friis <peavey@inspircd.org>
 *   Copyright (C) 2006-2008 Robin Burchell <robin+git@viroteck.net>
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


#include <random>

#ifndef _WIN32
# include <unistd.h>
#endif

#include "inspircd.h"
#include "timeutils.h"
#include "utility/string.h"
#include "xline.h"

extern "C" {
	// Rust FFI functions
	int helperfuncs_duration_try_from(const char* str, uint64_t* duration);
	uint64_t helperfuncs_duration_from(const char* str);
	int helperfuncs_duration_is_valid(const char* duration);
	char* helperfuncs_duration_to_string(uint64_t duration);
	char* helperfuncs_duration_to_long_string(uint64_t duration, int brief);
	char* helperfuncs_time_to_string(int64_t curtime, const char* format, int utc);
	void helperfuncs_strip_color(char* line);
	void helperfuncs_process_colors(char* line);
	void helperfuncs_free_string(char* ptr);
	int helperfuncs_is_sid(const char* sid);
	int helperfuncs_is_nick(const char* nick, size_t max_len);
	int helperfuncs_is_user(const char* user, size_t max_len);
	int helperfuncs_is_valid_mask(const char* mask, size_t max_len);
	int helperfuncs_is_host(const char* host, size_t max_len, int allowsimple);
	int helperfuncs_is_wordchar(int ch);
	uint64_t helperfuncs_gen_random_int(uint64_t max);
	char* helperfuncs_gen_random_str(size_t length);
	void helperfuncs_default_gen_random(uint8_t* output, size_t max);
}

bool InspIRCd::CheckPassword(const std::string& password, const std::string& passwordhash, const std::string& value)
{
	ModResult res;
	FIRST_MOD_RESULT(OnCheckPassword, res, (password, passwordhash, value));

	if (res == MOD_RES_ALLOW)
		return true; // Password explicitly valid.

	if (res == MOD_RES_DENY)
		return false; // Password explicitly invalid.

	// The hash algorithm wasn't recognised by any modules. If its plain
	// text then we can check it internally.
	if (passwordhash.empty() || insp::equalsci(passwordhash, "plaintext"))
		return TimingSafeCompare(password, value);

	// The password was invalid.
	return false;
}

bool InspIRCd::IsValidMask(const std::string& mask)
{
	// Delegate to Rust implementation
	char buffer[1025] = {0};
	const size_t len = std::min(mask.length(), sizeof(buffer) - 1);
	mask.copy(buffer, len);
	buffer[len] = '\0';
	
	return helperfuncs_is_valid_mask(buffer, ServerInstance->Config->Limits.GetMaxMask()) != 0;
}

void InspIRCd::StripColor(std::string& line)
{
	// Call Rust implementation
	helperfuncs_strip_color(line.data());
	line.resize(strlen(line.data()));
}

void InspIRCd::ProcessColors(std::vector<std::string>& input)
{
	for (auto& line : input)
		ProcessColors(line);
}

void InspIRCd::ProcessColors(std::string& line)
{
	// Call Rust implementation
	helperfuncs_process_colors(line.data());
	line.resize(strlen(line.data()));
}

/* true for valid nickname, false else */
bool InspIRCd::DefaultIsNick(const std::string_view& n)
{
	// Delegate to Rust implementation
	// Create a null-terminated temporary buffer for FFI
	char buffer[1025] = {0};
	const size_t len = std::min(n.length(), sizeof(buffer) - 1);
	n.copy(buffer, len);
	buffer[len] = '\0';
	
	return helperfuncs_is_nick(buffer, ServerInstance->Config->Limits.MaxNick) != 0;
}

/* return true for good username, false else */
bool InspIRCd::DefaultIsUser(const std::string_view& n)
{
	// Delegate to Rust implementation
	char buffer[1025] = {0};
	const size_t len = std::min(n.length(), sizeof(buffer) - 1);
	n.copy(buffer, len);
	buffer[len] = '\0';
	
	return helperfuncs_is_user(buffer, ServerInstance->Config->Limits.MaxUser) != 0;
}

bool InspIRCd::IsHost(const std::string_view& host, bool allowsimple)
{
	// Delegate to Rust implementation
	char buffer[1025] = {0};
	const size_t len = std::min(host.length(), sizeof(buffer) - 1);
	host.copy(buffer, len);
	buffer[len] = '\0';
	
	return helperfuncs_is_host(buffer, ServerInstance->Config->Limits.MaxHost, allowsimple ? 1 : 0) != 0;
}

bool InspIRCd::IsSID(const std::string_view& str)
{
	/* Returns true if the string given is exactly 3 characters long,
	 * starts with a digit, and the other two characters are A-Z or digits.
	 * This now delegates to the Rust implementation.
	 */
	// Create a null-terminated temporary buffer for FFI
	char buffer[4] = {0};
	const size_t len = std::min(str.length(), sizeof(buffer) - 1);
	str.copy(buffer, len);
	buffer[len] = '\0';
	
	return helperfuncs_is_sid(buffer) != 0;
}

namespace
{
	constexpr const auto SECONDS_PER_MINUTE = 60;

	constexpr const auto SECONDS_PER_HOUR = SECONDS_PER_MINUTE * 60;

	constexpr const auto SECONDS_PER_DAY = SECONDS_PER_HOUR * 24;

	constexpr const auto SECONDS_PER_WEEK = SECONDS_PER_DAY * 7;

	constexpr const auto SECONDS_PER_YEAR = (SECONDS_PER_DAY * 365);

	constexpr const auto SECONDS_PER_AVG_YEAR = SECONDS_PER_YEAR + (SECONDS_PER_HOUR * 6);
}

bool Duration::TryFrom(const std::string& str, unsigned long& duration)
{
	uint64_t rust_duration;
	if (helperfuncs_duration_try_from(str.c_str(), &rust_duration))
	{
		duration = static_cast<unsigned long>(rust_duration);
		return true;
	}
	return false;
}

unsigned long Duration::From(const std::string& str)
{
	return static_cast<unsigned long>(helperfuncs_duration_from(str.c_str()));
}

bool Duration::IsValid(const std::string& duration)
{
	return helperfuncs_duration_is_valid(duration.c_str()) != 0;
}

std::string Duration::ToString(unsigned long duration)
{
	char* result = helperfuncs_duration_to_string(static_cast<uint64_t>(duration));
	if (!result)
		return "0s";
	std::string str(result);
	helperfuncs_free_string(result);
	return str;
}

std::string Duration::ToLongString(unsigned long duration, bool brief)
{
	char* result = helperfuncs_duration_to_long_string(static_cast<uint64_t>(duration), brief ? 1 : 0);
	if (!result)
		return "0 seconds";
	std::string str(result);
	helperfuncs_free_string(result);
	return str;
}

std::string Time::ToString(time_t curtime, const char* format, bool utc)
{
	char* result = helperfuncs_time_to_string(static_cast<int64_t>(curtime), format, utc ? 1 : 0);
	if (!result)
		return "";
	std::string str(result);
	helperfuncs_free_string(result);
	return str;
}

std::string InspIRCd::GenRandomStr(size_t length) const
{
	// Delegate to Rust implementation
	char* rust_str = helperfuncs_gen_random_str(length);
	if (!rust_str)
		return "";
	std::string result(rust_str);
	helperfuncs_free_string(rust_str);
	return result;
}

std::string InspIRCd::GenRandomStr(size_t length, bool printable) const
{
	if (printable)
		return GenRandomStr(length);

	// DEPRECATED
	// Use Rust implementation for printable strings
	return GenRandomStr(length);
}

// NOTE: this has a slight bias for lower values if max is not a power of 2.
// Don't use it if that matters.
// Now uses Rust implementation for better randomness
unsigned long InspIRCd::GenRandomInt(unsigned long max) const
{
	if (max <= 1)
		return 0;
	return static_cast<unsigned long>(helperfuncs_gen_random_int(max));
}

// This is overridden by a higher-quality algorithm when TLS support is loaded
// Now delegates to Rust implementation
void InspIRCd::DefaultGenRandom(char* output, size_t max)
{
	// Try system entropy sources first for better randomness
#ifdef HAS_GETENTROPY
	if (getentropy(output, max) == 0)
		return;
#endif
#ifdef HAS_ARC4RANDOM_BUF
	arc4random_buf(output, max);
	return;
#endif
	// Use the Rust implementation which uses Xorshift algorithm
	helperfuncs_default_gen_random(reinterpret_cast<uint8_t*>(output), max);
}
