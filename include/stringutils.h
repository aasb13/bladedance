/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2013, 2021-2024 Sadie Powell <sadie@witchery.services>
 *   Copyright (C) 2013 Attila Molnar <attilamolnar@hush.com>
 *   Copyright (C) 2012 Robby <robby@chatbelgie.be>
 *   Copyright (C) 2008 Robin Burchell <robin+git@viroteck.net>
 *   Copyright (C) 2007 Dennis Friis <peavey@inspircd.org>
 *   Copyright (C) 2007 Craig Edwards <brain@inspircd.org>
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


#pragma once

#include <vector>
#include "utility/map.h"

/** C++ compatible std::string struct layout for Rust FFI. */
struct StdString {
	/** Pointer to heap-allocated data. */
	char* data;
	/** Length of the string. */
	size_t length;
	/** Capacity of the allocation. */
	size_t capacity;

	bool empty() const noexcept { return length == 0; }

	/** Converts this StdString to a std::string. */
	std::string to_std_string() const {
		if (!data || length == 0)
			return std::string();
		std::string result(data, length);
		return result;
	}
};

extern "C" {
	/** Destroys a StdString and frees its data pointer. */
	void StdString_Destroy(StdString* str);

	/** Encodes a byte array using percent encoding. */
	StdString Percent_Encode(const void* data, size_t length, const char* table, bool upper);
	/** Decodes a percent-encoded byte array. */
	StdString Percent_Decode(const void* data, size_t length);

	/** Encodes a byte array using hexadecimal encoding. */
	StdString Hex_Encode(const void* data, size_t length, const char* table, char separator);
	/** Decodes a hexadecimal-encoded byte array. */
	StdString Hex_Decode(const void* data, size_t length, const char* table, char separator);

	/** Encodes a byte array using Base64. */
	StdString Base64_Encode(const void* data, size_t length, const char* table, char padding);
	/** Decodes a Base64-encoded byte array. */
	StdString Base64_Decode(const void* data, size_t length, const char* table);

	/** Replaces template variables like %foo% within a string. */
	StdString Template_Replace(const char* str, size_t str_length,
		const char* const* vars_data, const char* const* vars_values, size_t vars_count);

	/** Timing-safe comparison of two strings to prevent timing attacks. */
	bool InspIRCd_TimingSafeCompare(const char* one, size_t one_length,
		const char* two, size_t two_length);

	/** Creates a new TokenList from a space-separated token list string. */
	void* TokenList_New(const char* tokenlist, size_t tokenlist_length);
	/** Destroys a TokenList instance. */
	void TokenList_Destroy(void* list);
	/** Adds a space-separated list of tokens to the TokenList. */
	void TokenList_AddList(void* list, const char* tokenlist, size_t tokenlist_length);
	/** Adds a token to the TokenList. */
	void TokenList_Add(void* list, const char* token, size_t token_length);
	/** Clears all tokens from the TokenList. */
	void TokenList_Clear(void* list);
	/** Checks if a token is contained in the TokenList. */
	bool TokenList_Contains(const void* list, const char* token, size_t token_length);
	/** Removes a token from the TokenList. */
	void TokenList_Remove(void* list, const char* token, size_t token_length);
	/** Converts the TokenList to a string representation. */
	StdString TokenList_ToString(const void* list);
	/** Compares two TokenLists for equality. */
	bool TokenList_Equals(const void* one, const void* two);
}

namespace Base64
{
	/** The default table used when handling Base64-encoded strings. */
	inline constexpr const char* TABLE = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

	/** Decodes a Base64-encoded byte array.
	 * @param data The byte array to decode from.
	 * @param length The length of the byte array.
	 * @param table The index table to use for decoding.
	 * @return The decoded form of the specified data.
	 */
	inline std::string Decode(const void* data, size_t length, const char* table = nullptr)
	{
		StdString result = Base64_Decode(data, length, table);
		std::string str = result.to_std_string();
		StdString_Destroy(&result);
		return str;
	}

	/** Decodes a Base64-encoded string.
	 * @param data The string to decode from.
	 * @param table The index table to use for decoding.
	 * @return The decoded form of the specified data.
	 */
	inline std::string Decode(const std::string& data, const char* table = nullptr)
	{
		return Decode(data.c_str(), data.length(), table);
	}

	/** Encodes a byte array using Base64.
	 * @param data The byte array to encode from.
	 * @param length The length of the byte array.
	 * @param table The index table to use for encoding.
	 * @param padding If non-zero then the character to pad encoded strings with.
	 * @return The encoded form of the specified data.
	 */
	inline std::string Encode(const void* data, size_t length, const char* table = nullptr, char padding = 0)
	{
		StdString result = Base64_Encode(data, length, table, padding);
		std::string str = result.to_std_string();
		StdString_Destroy(&result);
		return str;
	}

	/** Encodes a string using Base64.
	 * @param data The string to encode from.
	 * @param table The index table to use for encoding.
	 * @param padding If non-zero then the character to pad encoded strings with.
	 * @return The encoded form of the specified data.
	 */
	inline std::string Encode(const std::string& data, const char* table = nullptr, char padding = 0)
	{
		return Encode(data.c_str(), data.length(), table, padding);
	}
}

namespace Hex
{
	/** The table used for encoding as a lower-case hexadecimal string. */
	inline constexpr const char* TABLE_LOWER = "0123456789abcdef";

	/** The table used for encoding as an upper-case hexadecimal string. */
	inline constexpr const char* TABLE_UPPER = "0123456789ABCDEF";

	/** Decodes a hexadecimal-encoded byte array.
	 * @param data The byte array to decode from.
	 * @param length The length of the byte array.
	 * @param separator If non-zero then the character hexadecimal digits are separated with.
	 * @param table The index table to use for decoding.
	 * @return The decoded form of the specified data.
	 */
	inline std::string Decode(const void* data, size_t length, const char* table = nullptr, char separator = 0)
	{
		StdString result = Hex_Decode(data, length, table, separator);
		std::string str = result.to_std_string();
		StdString_Destroy(&result);
		return str;
	}

	/** Decodes a hexadecimal-encoded string.
	 * @param data The string to decode from.
	 * @param table The index table to use for decoding.
	 * @param separator If non-zero then the character hexadecimal digits are separated with.
	 * @return The decoded form of the specified data.
	 */
	inline std::string Decode(const std::string& data, const char* table = nullptr, char separator = 0)
	{
		return Decode(data.c_str(), data.length(), table, separator);
	}

	/** Encodes a byte array using hexadecimal encoding.
	 * @param data The byte array to encode from.
	 * @param length The length of the byte array.
	 * @param table The index table to use for encoding.
	 * @param separator If non-zero then the character to separate hexadecimal digits with.
	 * @return The encoded form of the specified data.
	 */
	inline std::string Encode(const void* data, size_t length, const char* table = nullptr, char separator = 0)
	{
		StdString result = Hex_Encode(data, length, table, separator);
		std::string str = result.to_std_string();
		StdString_Destroy(&result);
		return str;
	}

	/** Encodes a string using hexadecimal encoding.
	 * @param data The string to encode from.
	 * @param table The index table to use for encoding.
	 * @param separator If non-zero then the character to separate hexadecimal digits with.
	 * @return The encoded form of the specified data.
	 */
	inline std::string Encode(const std::string& data, const char* table = nullptr, char separator = 0)
	{
		return Encode(data.c_str(), data.length(), table, separator);
	}
}

namespace Percent
{
	/** The table used to determine what characters are safe within a percent-encoded string. */
	inline constexpr const char* TABLE = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.~";

	/** Decodes a percent-encoded byte array.
	 * @param data The byte array to decode from.
	 * @param length The length of the byte array.
	 * @return The decoded form of the specified data.
	 */
	inline std::string Decode(const void* data, size_t length)
	{
		StdString result = Percent_Decode(data, length);
		std::string str = result.to_std_string();
		StdString_Destroy(&result);
		return str;
	}

	/** Decodes a percent-encoded string.
	 * @param data The string to decode from.
	 * @return The decoded form of the specified data.
	 */
	inline std::string Decode(const std::string& data)
	{
		return Decode(data.c_str(), data.length());
	}

	/** Encodes a byte array using percent encoding.
	 * @param data The byte array to encode from.
	 * @param length The length of the byte array.
	 * @param table The table of characters that do not require escaping.
	 * @param upper Whether to use upper or lower case.
	 * @return The encoded form of the specified data.
	 */
	inline std::string Encode(const void* data, size_t length, const char* table = nullptr, bool upper = true)
	{
		StdString result = Percent_Encode(data, length, table, upper);
		std::string str = result.to_std_string();
		StdString_Destroy(&result);
		return str;
	}

	/** Encodes a string using percent encoding.
	 * @param data The string to encode from.
	 * @param table The table of characters that do not require escaping.
	 * @param upper Whether to use upper or lower case.
	 * @return The encoded form of the specified data.
	 */
	inline std::string Encode(const std::string& data, const char* table = nullptr, bool upper = true)
	{
		return Encode(data.c_str(), data.length(), table, upper);
	}
}

namespace Template
{
	/** A mapping of variable names to their values. */
	typedef insp::flat_map<std::string, std::string> VariableMap;

	/** Replaces template variables like %foo% within a string.
	 * @param str The string to template from.
	 * @param vars The variables to replace within the string.
	 * @return The specified string with all variables replaced within it.
	 */
	inline std::string Replace(const std::string& str, const VariableMap& vars)
	{
		// Convert the VariableMap to parallel arrays for C FFI.
		std::vector<const char*> names;
		std::vector<const char*> values;
		names.reserve(vars.size());
		values.reserve(vars.size());
		for (const auto& [name, value] : vars) {
			names.push_back(name.c_str());
			values.push_back(value.c_str());
		}

		StdString result = Template_Replace(str.c_str(), str.length(),
			names.empty() ? nullptr : names.data(),
			values.empty() ? nullptr : values.data(),
			vars.size());
		std::string output = result.to_std_string();
		StdString_Destroy(&result);
		return output;
	}
}

