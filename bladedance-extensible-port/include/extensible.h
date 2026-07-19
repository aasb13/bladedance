/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2013, 2019, 2021-2022 Sadie Powell <sadie@witchery.services>
 *   Copyright (C) 2012, 2014-2015 Attila Molnar <attilamolnar@hush.com>
 *   Copyright (C) 2012 Robby <robby@chatbelgie.be>
 *   Copyright (C) 2009 Daniel De Graaf <danieldg@inspircd.org>
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

// NOTE: The storage engine backing Extensible and ExtensionManager (the
// key->value map per instance, and the name->ExtensionItem registry) has
// been ported to Rust; see src/extensible.rs and src/extensible_glue.cpp.
// The classes below are now thin wrappers around that engine. The
// ExtensionItem hierarchy (include/extension.h) stays in C++ for now since
// it is templated on arbitrary value types and calls back into Module and
// Server, neither of which is ported yet.

#pragma once

#include <cstddef>
#include <string>
#include <utility>
#include <vector>

class ExtensionItem;

/** Types of extensible that an extension can extend. */
enum class ExtensionType
	: uint8_t
{
	/** The extension extends the User class. */
	USER = 0,

	/** The extension extends the Channel class. */
	CHANNEL = 1,

	/** The extension extends the Membership class. */
	MEMBERSHIP = 2,
};

// Rust FFI: see src/extensible.rs for the implementation of these.
extern "C"
{
	void* extensible_store_new();
	void extensible_store_free(void* store);
	size_t extensible_store_get_raw(const void* store, size_t key);
	size_t extensible_store_set_raw(void* store, size_t key, size_t value);
	size_t extensible_store_unset_raw(void* store, size_t key);
	bool extensible_store_is_empty(const void* store);
	size_t extensible_store_len(const void* store);
	bool extensible_store_entry_at(const void* store, size_t idx, size_t* out_key, size_t* out_value);
	void extensible_store_clear(void* store);

	void* extension_manager_new();
	void extension_manager_free(void* mgr);
	bool extension_manager_register(void* mgr, const char* name, size_t item, size_t creator);
	size_t extension_manager_get_item(const void* mgr, const char* name);
	size_t extension_manager_len(const void* mgr);
	bool extension_manager_entry_at(const void* mgr, size_t idx, size_t* out_item);
	size_t extension_manager_begin_unregister(void* mgr, size_t module_ptr, size_t* out_buf, size_t buf_len);
}

/** Base class for types which can be extended with additional data. */
class __attribute__ ((visibility ("default"))) Extensible
{
public:

	/** Allows extensions to access the extension store. */
	friend class ExtensionItem;

	/** The type of extensible that this is. */
	const ExtensionType extype:2;

	Extensible(const Extensible&) = delete;
	Extensible& operator=(const Extensible&) = delete;

	~Extensible();

	/** Frees all extensions attached to this extensible. */
	void FreeAllExtItems();

	/** Retrieves the values for extensions which are set on this extensible.
	 * Materializes a snapshot; prefer GetRawExt() for a single lookup.
	 */
	std::vector<std::pair<ExtensionItem*, void*>> GetExtList() const;

	/** Looks up the raw value for a single extension without materializing
	 * the whole list. Returns nullptr if unset.
	 * @param item The extension to look up.
	 */
	void* GetRawExt(const ExtensionItem* item) const;

	/** Unhooks the specifies extensions from this extensible.
	 * @param items The items to unhook.
	 */
	void UnhookExtensions(const std::vector<ExtensionItem*>& items);

protected:
	Extensible(ExtensionType exttype);

private:
	/** Rust-owned storage for the extensions set on this extensible. */
	void* store;

	/** Whether this extensible has been culled yet. */
	bool culled:1;
};

/** Manager for the extension system */
class __attribute__ ((visibility ("default"))) ExtensionManager final
{
public:
	ExtensionManager();
	~ExtensionManager();
	ExtensionManager(const ExtensionManager&) = delete;
	ExtensionManager& operator=(const ExtensionManager&) = delete;

	/** Begins unregistering extensions belonging to the specified module.
	 * @param module The module to unregister extensions for.
	 * @param list The list to add unregistered extensions to.
	 */
	void BeginUnregister(Module* module, std::vector<ExtensionItem*>& list);

	/** Retrieves all registered extensions. Each ExtensionItem carries its
	 * own registered name (ExtensionItem::name), so callers that need the
	 * name can read it directly off the returned pointer.
	 */
	std::vector<ExtensionItem*> GetExts() const;

	/** Retrieves an extension by name.
	 * @param name The name of the extension to retrieve.
	 * @return Either the value of this extension or nullptr if it does not exist.
	 */
	ExtensionItem* GetItem(const std::string& name);

	/** Registers an extension with the manager.
	 * @return Either true if the extension was registered or false if an extension with the same
	 *         name already exists.
	 */
	bool Register(ExtensionItem* item);

private:
	/** Rust-owned registry of registered extensions. */
	void* store;
};
