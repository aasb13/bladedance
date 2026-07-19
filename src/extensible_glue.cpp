/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2013, 2019, 2021-2022 Sadie Powell <sadie@witchery.services>
 *   Copyright (C) 2012, 2014-2015 Attila Molnar <attilamolnar@hush.com>
 *   Copyright (C) 2012 Robby <robby@chatbelgie.be>
 *   Copyright (C) 2009 Daniel De Graaf <danieldg@inspircd.org>
 *   Copyright (C) 2019-2024 Sadie Powell <sadie@witchery.services>
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

// This file is the C++ shell wrapping the Rust storage engine in
// src/extensible.rs. Extensible/ExtensionManager themselves no longer own a
// C++ container; they hold an opaque pointer into Rust-owned memory and
// delegate all map operations across the FFI boundary. The ExtensionItem
// virtual-dispatch hierarchy (include/extension.h, unchanged) still lives
// in C++ and is called back into from here wherever behavior (Delete,
// serialization) is needed, since that behavior is arbitrary per-module
// C++ code.

#include "inspircd.h"
#include "extension.h"

// ---------------------------------------------------------------------
// Extensible
// ---------------------------------------------------------------------

Extensible::Extensible(ExtensionType exttype)
	: extype(exttype)
	, store(extensible_store_new())
	, culled(false)
{
}

Extensible::~Extensible()
{
	if ((!extensible_store_is_empty(store) || !culled) && ServerInstance)
	{
		ServerInstance->Logs.Debug("CULL", "Extensible was deleted without being culled: @{}",
			fmt::ptr(this));
	}
	extensible_store_free(store);
}

void Extensible::FreeAllExtItems()
{
	const size_t count = extensible_store_len(store);
	for (size_t i = 0; i < count; ++i)
	{
		size_t key = 0;
		size_t value = 0;
		if (!extensible_store_entry_at(store, i, &key, &value))
			break;

		auto* extension = reinterpret_cast<ExtensionItem*>(key);
		extension->Delete(this, reinterpret_cast<void*>(value));
	}
	extensible_store_clear(store);
}

std::vector<std::pair<ExtensionItem*, void*>> Extensible::GetExtList() const
{
	std::vector<std::pair<ExtensionItem*, void*>> result;
	const size_t count = extensible_store_len(store);
	result.reserve(count);
	for (size_t i = 0; i < count; ++i)
	{
		size_t key = 0;
		size_t value = 0;
		if (!extensible_store_entry_at(store, i, &key, &value))
			break;

		result.emplace_back(reinterpret_cast<ExtensionItem*>(key), reinterpret_cast<void*>(value));
	}
	return result;
}

void* Extensible::GetRawExt(const ExtensionItem* item) const
{
	return reinterpret_cast<void*>(extensible_store_get_raw(store, reinterpret_cast<size_t>(item)));
}

void Extensible::UnhookExtensions(const std::vector<ExtensionItem*>& items)
{
	// Note: every ExtensionItem in this codebase only ever stores a
	// non-zero raw value when "set" (Unset()/UnsetRaw() always erases the
	// entry rather than storing a zero sentinel), so a 0 read here reliably
	// means "nothing set for this item".
	for (auto* item : items)
	{
		const size_t key = reinterpret_cast<size_t>(item);
		const size_t value = extensible_store_unset_raw(store, key);
		if (value != 0)
			item->Delete(this, reinterpret_cast<void*>(value));
	}
}

// ---------------------------------------------------------------------
// ExtensionManager
// ---------------------------------------------------------------------

ExtensionManager::ExtensionManager()
	: store(extension_manager_new())
{
}

ExtensionManager::~ExtensionManager()
{
	extension_manager_free(store);
}

bool ExtensionManager::Register(ExtensionItem* item)
{
	Module* creator = item->creator;
	return extension_manager_register(
		store,
		item->name.c_str(),
		reinterpret_cast<size_t>(item),
		reinterpret_cast<size_t>(creator));
}

void ExtensionManager::BeginUnregister(Module* module, std::vector<ExtensionItem*>& items)
{
	const size_t capacity = extension_manager_len(store);
	std::vector<size_t> buf(capacity);
	const size_t removed = extension_manager_begin_unregister(
		store, reinterpret_cast<size_t>(module), buf.data(), buf.size());

	items.reserve(items.size() + removed);
	for (size_t i = 0; i < removed; ++i)
		items.push_back(reinterpret_cast<ExtensionItem*>(buf[i]));
}

std::vector<ExtensionItem*> ExtensionManager::GetExts() const
{
	std::vector<ExtensionItem*> result;
	const size_t count = extension_manager_len(store);
	result.reserve(count);
	for (size_t i = 0; i < count; ++i)
	{
		size_t item = 0;
		if (!extension_manager_entry_at(store, i, &item))
			break;
		result.push_back(reinterpret_cast<ExtensionItem*>(item));
	}
	return result;
}

ExtensionItem* ExtensionManager::GetItem(const std::string& name)
{
	const size_t item = extension_manager_get_item(store, name.c_str());
	return reinterpret_cast<ExtensionItem*>(item);
}

// ---------------------------------------------------------------------
// ExtensionItem and its concrete subclasses: unchanged from upstream.
// These stay in C++ because they are templated / virtual-dispatch classes
// that call back into Module and Server, neither of which is ported to
// Rust yet. They already go through Extensible::GetRawExt (formerly a
// private GetRaw/SetRaw/UnsetRaw against the raw map) via the public
// wrapper API added above.
// ---------------------------------------------------------------------

ExtensionItem::ExtensionItem(Module* mod, const std::string& Key, ExtensionType exttype)
	: ServiceProvider(mod, Key, SERVICE_METADATA)
	, extype(exttype)
{
}

void ExtensionItem::OnSync(const Extensible* container, void* item, Server* server)
{
}

void ExtensionItem::RegisterService()
{
	if (!ServerInstance->Extensions.Register(this))
		throw ModuleException(creator, "Extension already exists: " + name);
}

void* ExtensionItem::GetRaw(const Extensible* container) const
{
	return container->GetRawExt(this);
}

void* ExtensionItem::SetRaw(Extensible* container, void* value)
{
	const size_t key = reinterpret_cast<size_t>(this);
	const size_t old = extensible_store_set_raw(container->store, key, reinterpret_cast<size_t>(value));
	return reinterpret_cast<void*>(old);
}

void* ExtensionItem::UnsetRaw(Extensible* container)
{
	const size_t key = reinterpret_cast<size_t>(this);
	const size_t old = extensible_store_unset_raw(container->store, key);
	return reinterpret_cast<void*>(old);
}

void ExtensionItem::Sync(const Extensible* container, void* item)
{
	const std::string networkstr = item ? ToNetwork(container, item) : "";
	ServerInstance->PI->SendMetadata(container, name, networkstr);
	OnSync(container, item, nullptr);
}

void ExtensionItem::FromInternal(Extensible* container, const std::string& value) noexcept
{
}

void ExtensionItem::FromNetwork(Extensible* container, const std::string& value) noexcept
{
}

std::string ExtensionItem::ToHuman(const Extensible* container, void* item) const noexcept
{
	// Try to use the network form by default.
	std::string ret = ToNetwork(container, item);

	// If there's no network form then fall back to the internal form.
	if (ret.empty())
		ret = ToInternal(container, item);

	return ret;
}

std::string ExtensionItem::ToInternal(const Extensible* container, void* item) const noexcept
{
	return {};
}

std::string ExtensionItem::ToNetwork(const Extensible* container, void* item) const noexcept
{
	return {};
}

BoolExtItem::BoolExtItem(Module* owner, const std::string& key, ExtensionType exttype, bool sync)
	: ExtensionItem(owner, key, exttype)
	, synced(sync)
{
}

void BoolExtItem::Delete(Extensible* container, void* item)
{
	// Intentionally left blank.
}

void BoolExtItem::FromInternal(Extensible* container, const std::string& value) noexcept
{
	if (ConvToNum<intptr_t>(value))
		Set(container, false);
	else
		Unset(container, false);
}

std::string BoolExtItem::ToHuman(const Extensible* container, void* item) const noexcept
{
	return item ? "set" : "unset";
}

void BoolExtItem::FromNetwork(Extensible* container, const std::string& value) noexcept
{
	if (synced)
		FromInternal(container, value);
}

std::string BoolExtItem::ToInternal(const Extensible* container, void* item) const noexcept
{
	return ConvToStr(!!item);
}

std::string BoolExtItem::ToNetwork(const Extensible* container, void* item) const noexcept
{
	return synced ? ToInternal(container, item) : std::string();
}

bool BoolExtItem::Get(const Extensible* container) const
{
	return GetRaw(container);
}

void BoolExtItem::Set(Extensible* container, bool sync)
{
	if (container->extype != this->extype)
		return;

	SetRaw(container, reinterpret_cast<void*>(1));
	if (sync && synced)
		Sync(container, reinterpret_cast<void*>(1));
}

void BoolExtItem::Unset(Extensible* container, bool sync)
{
	if (container->extype != this->extype)
		return;

	UnsetRaw(container);
	if (sync && synced)
		Sync(container, reinterpret_cast<void*>(0));
}

IntExtItem::IntExtItem(Module* owner, const std::string& key, ExtensionType exttype, bool sync)
	: ExtensionItem(owner, key, exttype)
	, synced(sync)
{
}

void IntExtItem::Delete(Extensible* container, void* item)
{
	// Intentionally left blank.
}

void IntExtItem::FromInternal(Extensible* container, const std::string& value) noexcept
{
	Set(container, ConvToNum<intptr_t>(value), false);
}

void IntExtItem::FromNetwork(Extensible* container, const std::string& value) noexcept
{
	if (synced)
		FromInternal(container, value);
}

intptr_t IntExtItem::Get(const Extensible* container) const
{
	return reinterpret_cast<intptr_t>(GetRaw(container));
}

void IntExtItem::Set(Extensible* container, intptr_t value, bool sync)
{
	if (container->extype != this->extype)
		return;

	if (value)
		SetRaw(container, reinterpret_cast<void*>(value));
	else
		UnsetRaw(container);

	if (sync && synced)
		Sync(container, GetRaw(container));
}

std::string IntExtItem::ToInternal(const Extensible* container, void* item) const noexcept
{
	return ConvToStr(reinterpret_cast<intptr_t>(item));
}

std::string IntExtItem::ToNetwork(const Extensible* container, void* item) const noexcept
{
	return synced ? ToInternal(container, item) : std::string();
}

void IntExtItem::Unset(Extensible* container, bool sync)
{
	if (container->extype != this->extype)
		return;

	UnsetRaw(container);
	if (sync && synced)
		Sync(container, nullptr);
}

StringExtItem::StringExtItem(Module* owner, const std::string& key, ExtensionType exttype, bool sync)
	: SimpleExtItem(owner, key, exttype, sync)
{
}

void StringExtItem::FromInternal(Extensible* container, const std::string& value) noexcept
{
	if (value.empty())
		Unset(container, false);
	else
		Set(container, value, false);
}

std::string StringExtItem::ToInternal(const Extensible* container, void* item) const noexcept
{
	return item ? *static_cast<std::string*>(item) : std::string();
}
