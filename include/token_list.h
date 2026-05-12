/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2023 Sadie Powell <sadie@witchery.services>
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

#include "compat.h"
#include "stringutils.h"

/** Encapsulates a list of tokens in the format "* -FOO -BAR".*/
class CoreExport TokenList final
{
private:
	/** Opaque pointer to the Rust TokenList instance. */
	void* ptr = nullptr;

public:
	/** Creates a new empty token list. */
	TokenList() { ptr = TokenList_New(nullptr, 0); }

	/** Creates a new token list from a list of tokens. */
	TokenList(const std::string& tokenlist) { ptr = TokenList_New(tokenlist.c_str(), tokenlist.length()); }

	/** Copy constructor. */
	TokenList(const TokenList& other) {
		StdString str = TokenList_ToString(other.ptr);
		ptr = TokenList_New(str.data, str.length);
		StdString_Destroy(&str);
	}

	/** Move constructor. */
	TokenList(TokenList&& other) noexcept : ptr(other.ptr) { other.ptr = nullptr; }

	/** Assignment operator. */
	TokenList& operator=(const TokenList& other) {
		if (this != &other) {
			TokenList_Destroy(ptr);
			StdString str = TokenList_ToString(other.ptr);
			ptr = TokenList_New(str.data, str.length);
			StdString_Destroy(&str);
		}
		return *this;
	}

	/** Move assignment operator. */
	TokenList& operator=(TokenList&& other) noexcept {
		if (this != &other) {
			TokenList_Destroy(ptr);
			ptr = other.ptr;
			other.ptr = nullptr;
		}
		return *this;
	}

	/** Destructor. */
	~TokenList() { TokenList_Destroy(ptr); }

	/** Adds a space-delimited list of tokens to the token list.
	 * @param tokenlist The list of space-delimited tokens to add.
	 */
	void AddList(const std::string& tokenlist) {
		TokenList_AddList(ptr, tokenlist.c_str(), tokenlist.length());
	}

	/** Adds a single token to the token list.
	 * @param token The token to add.
	 */
	void Add(const std::string& token) {
		TokenList_Add(ptr, token.c_str(), token.length());
	}

	/** Removes all tokens from the token list. */
	void Clear() {
		TokenList_Clear(ptr);
	}

	/** Determines whether the specified token exists in the token list.
	 * @param token The token to search for.
	 */
	bool Contains(const std::string& token) const {
		return TokenList_Contains(ptr, token.c_str(), token.length());
	}

	/** Removes the specified token from the token list.
	 * @param token The token to remove.
	 */
	void Remove(const std::string& token) {
		TokenList_Remove(ptr, token.c_str(), token.length());
	}

	/** Retrieves a string which represents the contents of this token list. */
	std::string ToString() const {
		StdString result = TokenList_ToString(ptr);
		std::string str = result.to_std_string();
		StdString_Destroy(&result);
		return str;
	}

	/** Determines whether the specified token list contains the same tokens as this instance.
	 * @param other The tokenlist to compare against.
	 * @return True if the token lists are equal; otherwise, false.
	 */
	bool operator==(const TokenList& other) const {
		return TokenList_Equals(ptr, other.ptr);
	}
};
