/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2013-2026 Sadie Powell <sadie@witchery.services>
 *   Copyright (C) 2012-2014 Attila Molnar <attilamolnar@hush.com>
 *   Copyright (C) 2009-2010 Daniel De Graaf <danieldg@inspircd.org>
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


#include <cinttypes>
#include <cstdlib>
#include <functional>

#include "inspircd.h"
#include "configparser.h"
#include "utility/string.h"

// Helper function for ConfigTag methods
static inline void CheckRangeSimple(const ConfigTag* tag, const std::string& key, uintmax_t& num, uintmax_t def, uintmax_t min, uintmax_t max)
{
	if (num < min || num > max)
	{
		tag->LogMalformed(key, std::to_string(num), std::to_string(def), "not between " + std::to_string(min) + " and " + std::to_string(max));
		num = def;
	}
}

static inline void CheckRangeSimple(const ConfigTag* tag, const std::string& key, long double& num, long double def, long double min, long double max)
{
	if (num < min || num > max)
	{
		tag->LogMalformed(key, std::to_string(num), std::to_string(def), "not between " + std::to_string(min) + " and " + std::to_string(max));
		num = def;
	}
}

static inline void CheckRangeSimple(const ConfigTag* tag, const std::string& key, intmax_t& num, intmax_t def, intmax_t min, intmax_t max)
{
	if (num < min || num > max)
	{
		tag->LogMalformed(key, std::to_string(num), std::to_string(def), "not between " + std::to_string(min) + " and " + std::to_string(max));
		num = def;
	}
}

bool ConfigTag::readString(const std::string& key, std::string& value, bool allow_lf) const
{
	auto it = items.find(key);
	if (it == items.end())
		return false;

	value = it->second;
	if (!allow_lf && (value.find('\n') != std::string::npos))
	{
		for (auto& chr : value)
		{
			if (chr == '\n')
				chr = ' ';
		}
	}
	return true;
}

void ConfigTag::LogMalformed(const std::string& key, const std::string& val, const std::string& def, const std::string& reason) const
{
	::Logs.Warning("CONFIG", "The value of <{}:{}> at {} ({}) is {}; using the default ({}) instead.",
		name, key, source.str(), val, reason, def);
}

// ConfigTag methods - these read from the items map which is populated from Rust TOML data
// They do NOT perform C++ XML parsing at all

intmax_t ConfigTag::getSInt(const std::string& key, intmax_t def, intmax_t min, intmax_t max) const
{
	std::string result;
	if(!readString(key, result) || result.empty())
		return def;

	const char* res_cstr = result.c_str();
	char* res_tail = nullptr;
	intmax_t res = strtoimax(res_cstr, &res_tail, 0);
	if (res_tail == res_cstr)
		return def;

	// NO magnitude checking - TOML uses direct values only
	CheckRangeSimple(this, key, res, def, min, max);
	return res;
}

uintmax_t ConfigTag::getUInt(const std::string& key, uintmax_t def, uintmax_t min, uintmax_t max) const
{
	std::string result;
	if (!readString(key, result) || result.empty())
		return def;

	const char* res_cstr = result.c_str();
	char* res_tail = nullptr;
	uintmax_t res = strtoumax(res_cstr, &res_tail, 0);
	if (res_tail == res_cstr)
		return def;

	// NO magnitude checking - TOML uses direct values only
	CheckRangeSimple(this, key, res, def, min, max);
	return res;
}

long double ConfigTag::getFloat(const std::string& key, long double def, long double min, long double max) const
{
	std::string result;
	if (!readString(key, result))
		return def;

	long double res = strtold(result.c_str(), nullptr);
	CheckRangeSimple(this, key, res, def, min, max);
	return res;
}

unsigned long ConfigTag::getDuration(const std::string& key, unsigned long def, unsigned long min, unsigned long max) const
{
	std::string result;
	if (!readString(key, result) || result.empty())
		return def;

	const char* res_cstr = result.c_str();
	char* res_tail = nullptr;
	unsigned long res = strtoul(res_cstr, &res_tail, 0);
	if (res_tail == res_cstr)
		return def;

	// NO magnitude checking - TOML uses direct values only
	if (res < min || res > max)
	{
		LogMalformed(key, result, std::to_string(def), "not between " + std::to_string(min) + " and " + std::to_string(max));
		return def;
	}
	return res;
}

bool ConfigTag::getBool(const std::string& key, bool def) const
{
	std::string result;
	if (!readString(key, result) || result.empty())
		return def;

	if (insp::equalsci(result, "yes") || insp::equalsci(result, "true") || insp::equalsci(result, "on"))
		return true;
	else if (insp::equalsci(result, "no") || insp::equalsci(result, "false") || insp::equalsci(result, "off"))
		return false;
	else
		return def;
}

std::string ConfigTag::getString(const std::string& key, const std::string& def, const std::function<bool(const std::string&)>& validator) const
{
	std::string result;
	if (!readString(key, result))
		return def;

	if (validator && !validator(result))
		return def;

	return result;
}

std::string ConfigTag::getString(const std::string& key, const std::string& def, size_t minlen, size_t maxlen) const
{
	std::string result;
	if (!readString(key, result))
		return def;

	if (result.length() < minlen || result.length() > maxlen)
		return def;

	return result;
}

unsigned char ConfigTag::getCharacter(const std::string& key, unsigned char def, bool emptynul) const
{
	std::string result;
	if (!readString(key, result) || result.empty())
		return def;

	if (result.length() != 1 && !(emptynul && result.empty()))
		return def;

	return result[0];
}


