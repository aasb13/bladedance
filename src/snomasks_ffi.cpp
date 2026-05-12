/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2021-2025 Sadie Powell <sadie@witchery.services>
 *
 * FFI for snomasks Rust port.
 */

#include "inspircd.h"

Snomask* SnomaskRustAccess::Mask(SnomaskManager* mgr, size_t idx)
{
	return &mgr->masks[idx];
}

std::string& SnomaskRustAccess::Description(Snomask* s)
{
	return s->Description;
}

std::string& SnomaskRustAccess::LastMessage(Snomask* s)
{
	return s->LastMessage;
}

char& SnomaskRustAccess::LastLetter(Snomask* s)
{
	return s->LastLetter;
}

unsigned int& SnomaskRustAccess::Count(Snomask* s)
{
	return s->Count;
}

void Snomask::Send(char letter, const std::string& desc, const std::string& msg)
{
	ServerInstance->Logs.Normal(desc, msg);
	const std::string finalmsg = INSP_FORMAT("*** {}: {}", desc, msg);

	for (auto* user : ServerInstance->Users.all_opers)
	{
		if (IS_LOCAL(user) && user->IsNoticeMaskSet(letter))
			user->WriteNotice(finalmsg);
	}
}

void SnomaskRustAccess::Send(char letter, const std::string& desc, const std::string& msg)
{
	Snomask::Send(letter, desc, msg);
}

extern "C" {

void snomask_ffi_description_set(SnomaskManager* mgr, size_t slot, const char* text)
{
	if (slot >= 26)
		return;
	SnomaskRustAccess::Description(SnomaskRustAccess::Mask(mgr, slot)) = text;
}

Snomask* snomask_ffi_mask(SnomaskManager* mgr, size_t slot)
{
	if (slot >= 26)
		return nullptr;
	return SnomaskRustAccess::Mask(mgr, slot);
}

const char* snomask_ffi_description_cstr(Snomask* s)
{
	static thread_local std::string tls;
	tls = SnomaskRustAccess::Description(s);
	return tls.c_str();
}

void snomask_ffi_last_message_assign(Snomask* s, const char* v)
{
	SnomaskRustAccess::LastMessage(s) = v;
}

void snomask_ffi_last_message_clear(Snomask* s)
{
	SnomaskRustAccess::LastMessage(s).clear();
}

const char* snomask_ffi_last_message_cstr(Snomask* s)
{
	static thread_local std::string tls;
	tls = SnomaskRustAccess::LastMessage(s);
	return tls.c_str();
}

void snomask_ffi_last_letter_set(Snomask* s, char c)
{
	SnomaskRustAccess::LastLetter(s) = c;
}

char snomask_ffi_last_letter_get(Snomask* s)
{
	return SnomaskRustAccess::LastLetter(s);
}

unsigned int snomask_ffi_count_get(Snomask* s)
{
	return SnomaskRustAccess::Count(s);
}

void snomask_ffi_count_set(Snomask* s, unsigned int v)
{
	SnomaskRustAccess::Count(s) = v;
}

bool snomask_ffi_no_snotice_stack()
{
	return ServerInstance->Config->NoSnoticeStack;
}

bool snomask_ffi_first_mod_on_send_snotice(char letter, const char* desc, const char* msg)
{
	std::string d(desc), m(msg);
	ModResult modres;
	FIRST_MOD_RESULT(OnSendSnotice, modres, (letter, d, m));
	return modres == MOD_RES_DENY;
}

void snomask_ffi_foreach_mod_on_send_snotice(char letter, const char* desc, const char* msg)
{
	std::string d(desc), m(msg);
	FOREACH_MOD(OnSendSnotice, (letter, d, m));
}

void snomask_ffi_send_impl(char letter, const char* desc, const char* msg)
{
	SnomaskRustAccess::Send(letter, std::string(desc), std::string(msg));
}

void snomask_ffi_send_global_notice(char letter, const char* text)
{
	ServerInstance->PI->SendSNONotice(static_cast<char>(toupper(static_cast<unsigned char>(letter))), text);
}

} // extern "C"
