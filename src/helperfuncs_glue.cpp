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
	const char* dest = mask.c_str();
	int exclamation = 0;
	int atsign = 0;

	for (const char* i = dest; *i; i++)
	{
		/* out of range character, bad mask */
		if (*i < 32 || *i > 126)
		{
			return false;
		}

		switch (*i)
		{
			case '!':
				exclamation++;
				break;
			case '@':
				atsign++;
				break;
		}
	}

	/* valid masks only have 1 ! and @ */
	if (exclamation != 1 || atsign != 1)
		return false;

	if (mask.length() > ServerInstance->Config->Limits.GetMaxMask())
		return false;

	return true;
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
	if (n.empty() || n.length() > ServerInstance->Config->Limits.MaxUser)
		return false;

	for (const auto chr : n)
	{
		if (chr >= 'A' && chr <= '}')
			continue;

		if ((chr >= '0' && chr <= '9') || chr == '-' || chr == '.')
			continue;

		return false;
	}

	return true;
}

bool InspIRCd::IsHost(const std::string_view& host, bool allowsimple)
{
	// Hostnames must be non-empty and shorter than the maximum hostname length.
	if (host.empty() || host.length() > ServerInstance->Config->Limits.MaxHost)
		return false;

	unsigned int numdashes = 0;
	unsigned int numdots = 0;
	bool seendot = false;
	const auto hostend = host.end() - 1;
	for (auto iter = host.begin(); iter != host.end(); ++iter)
	{
		const auto chr = static_cast<unsigned char>(*iter);

		// If the current character is a label separator.
		if (chr == '.')
		{
			numdots++;

			// Consecutive separators are not allowed and dashes can not exist at the start or end
			// of labels and separators must only exist between labels.
			if (seendot || numdashes || iter == host.begin() || iter == hostend)
				return false;

			seendot = true;
			continue;
		}

		// If this point is reached then the character is not a dot.
		seendot = false;

		// If the current character is a dash.
		if (chr == '-')
		{
			// Consecutive separators are not allowed and dashes can not exist at the start or end
			// of labels and separators must only exist between labels.
			if (seendot || numdashes >= 2 || iter == host.begin() || iter == hostend)
				return false;

			numdashes += 1;
			continue;
		}

		// If this point is reached then the character is not a dash.
		numdashes = 0;

		// Alphanumeric characters are allowed at any position.
		if ((chr >= '0' && chr <= '9') || (chr >= 'A' && chr <= 'Z') || (chr >= 'a' && chr <= 'z'))
			continue;

		return false;
	}

	// Whilst simple hostnames (e.g. localhost) are valid we do not allow the server to use
	// them to prevent issues with clients that differentiate between short client and server
	// prefixes by checking whether the nickname contains a dot.
	return numdots || allowsimple;
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
	static const char chars[] = {
		'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm',
		'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
		'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M',
		'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
		'0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
	};

	std::string buf;
	buf.reserve(length);
	for (size_t idx = 0; idx < length; ++idx)
		buf.push_back(chars[GenRandomInt(std::size(chars))]);
	return buf;
}

std::string InspIRCd::GenRandomStr(size_t length, bool printable) const
{
	if (printable)
		return GenRandomStr(length);

	// DEPRECATED
	std::vector<char> str(length);
	GenRandom(str.data(), length);
	return std::string(str.data(), str.size());
}

// NOTE: this has a slight bias for lower values if max is not a power of 2.
// Don't use it if that matters.
unsigned long InspIRCd::GenRandomInt(unsigned long max) const
{
	unsigned long rv;
	GenRandom(reinterpret_cast<char*>(&rv), sizeof(rv));
	return rv % max;
}

// This is overridden by a higher-quality algorithm when TLS support is loaded
void InspIRCd::DefaultGenRandom(char* output, size_t max)
{
#ifdef HAS_GETENTROPY
	if (getentropy(output, max) == 0)
		return;
#endif
#ifdef HAS_ARC4RANDOM_BUF
	arc4random_buf(output, max);
#else
	static std::random_device device;
	static std::mt19937 engine(device());
	static std::uniform_int_distribution<short> dist(CHAR_MIN, CHAR_MAX);
	for (size_t i = 0; i < max; ++i)
		output[i] = static_cast<char>(dist(engine));
#endif
}
