/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2021-2025 Sadie Powell <sadie@witchery.services>
 */

#include "inspircd.h"

extern "C" {
void rust_snomask_manager_ctor_init(SnomaskManager* m);
void rust_snomask_flush_snotices(SnomaskManager* m);
void rust_snomask_enable(SnomaskManager* m, char letter, const char* desc);
void rust_snomask_write_to_mask(SnomaskManager* m, char letter, const char* text);
void rust_snomask_write_global_sno(SnomaskManager* m, char letter, const char* text);
bool rust_snomask_is_snomask(char ch);
bool rust_snomask_is_usable(const SnomaskManager* m, char ch);
}

SnomaskManager::SnomaskManager()
{
	rust_snomask_manager_ctor_init(this);
}

void SnomaskManager::FlushSnotices()
{
	rust_snomask_flush_snotices(this);
}

void SnomaskManager::EnableSnomask(char letter, const std::string& type)
{
	rust_snomask_enable(this, letter, type.c_str());
}

void SnomaskManager::WriteToSnoMask(char letter, const std::string& text)
{
	rust_snomask_write_to_mask(this, letter, text.c_str());
}

void SnomaskManager::WriteGlobalSno(char letter, const std::string& text)
{
	rust_snomask_write_global_sno(this, letter, text.c_str());
}

bool SnomaskManager::IsSnomask(char ch)
{
	return rust_snomask_is_snomask(ch);
}

bool SnomaskManager::IsSnomaskUsable(char ch) const
{
	return rust_snomask_is_usable(this, ch);
}
