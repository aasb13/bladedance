/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2025 Account API Glue
 *
 * This file is part of InspIRCd.  InspIRCd is free software: you can
 * redistribute it and/or modify it under terms of the GNU General Public
 * License as published by Free Software Foundation, version 2.
 *
 * This program is distributed in hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE.  See the GNU General Public License for more
 * details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

#include "inspircd.h"
#include "extension.h"
#include "modules/account.h"

extern "C" CoreExport char* account_ffi_get_account_name(User* user) {
	if (!user) {
		return nullptr;
	}
	
	// Direct access to account extension if available
	// Check if the user has an account extension set
	ExtensionItem* account_ext = ServerInstance->Extensions.GetItem("accountname");
	if (!account_ext) {
		return nullptr;
	}
	
	// Get the account name from the extension
	StringExtItem* string_ext = static_cast<StringExtItem*>(account_ext);
	std::string* account_name = string_ext->Get(user);
	if (!account_name || account_name->empty()) {
		return nullptr;
	}
	
	// Allocate memory for the C string and copy the content
	char* c_str = new char[account_name->length() + 1];
	std::strcpy(c_str, account_name->c_str());
	return c_str;
}

extern "C" CoreExport void account_ffi_free_string(char* ptr) {
	if (ptr) {
		delete[] ptr;
	}
}
