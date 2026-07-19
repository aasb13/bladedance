/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2021 Mistral AI
 *   Copyright (C) 2017, 2019, 2022-2023 Sadie Powell <sadie@witchery.services>
 *   Copyright (C) 2013-2015 Attila Molnar <attilamolnar@hush.com>
 *   Copyright (C) 2012 ChrisTX <xpipe@hotmail.de>
 *   Copyright (C) 2011, 2014 Adam <Adam@anope.org>
 *   Copyright (C) 2009-2010 Daniel De Graaf <danieldg@inspircd.org>
 *   Copyright (C) 2008 Robin Burchell <robin+git@viroteck.net>
 *   Copyright (C) 2007 Dennis Friis <peavey@inspircd.org>
 *   Copyright (C) 2006-2008 Craig Edwards <brain@inspircd.org>
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


#include "inspircd.h"

#ifndef _WIN32
#include <sys/select.h>
#endif // _WIN32

/** Rust FFI declarations for select socket engine */
extern "C" bool rust_socketengine_select_init();
extern "C" void rust_socketengine_select_deinit();
extern "C" bool rust_socketengine_select_recover_from_fork();
extern "C" bool rust_socketengine_select_add_fd(int fd, int event_mask, void* eh_ptr);
extern "C" bool rust_socketengine_select_mod_fd(int fd, int old_mask, int new_mask, void* eh_ptr);
extern "C" bool rust_socketengine_select_del_fd(int fd);
extern "C" int rust_socketengine_select_wait(int timeout_sec, int timeout_usec);
extern "C" int rust_socketengine_select_get_max_fd();
extern "C" int rust_socketengine_select_has_events(int fd, const fd_set* rfdset, const fd_set* wfdset, const fd_set* errfdset);

/** A specialisation of the SocketEngine class, designed to use traditional select().
 */
namespace
{
	fd_set ReadSet, WriteSet, ErrSet;
	int MaxFD = 0;
}

void SocketEngine::Init()
{
#ifdef _WIN32
	// Set up winsock.
	WSADATA wsadata;
	WSAStartup(MAKEWORD(2, 2), &wsadata);
#endif

	MaxSetSize = FD_SETSIZE;

	// Initialize Rust select engine
	rust_socketengine_select_init();

	FD_ZERO(&ReadSet);
	FD_ZERO(&WriteSet);
	FD_ZERO(&ErrSet);
}

void SocketEngine::Deinit()
{
	// Clean up Rust select engine
	rust_socketengine_select_deinit();
}

void SocketEngine::RecoverFromFork()
{
	// Recover Rust select engine after fork
	rust_socketengine_select_recover_from_fork();
}

bool SocketEngine::AddFd(EventHandler* eh, int event_mask)
{
	if (!eh->HasFd())
		return false;

	int fd = eh->GetFd();
	if (static_cast<size_t>(fd) >= GetMaxFds())
		return false;

	if (!SocketEngine::AddFdRef(eh))
		return false;

	// Use Rust implementation
	if (!rust_socketengine_select_add_fd(fd, event_mask, static_cast<void*>(eh)))
		return false;

	eh->SetEventMask(event_mask);
	OnSetEvent(eh, 0, event_mask);
	FD_SET(fd, &ErrSet);
	if (fd > MaxFD)
		MaxFD = fd;

	ServerInstance->Logs.Debug("SOCKET", "New file descriptor: {}", fd);
	return true;
}

void SocketEngine::DelFd(EventHandler* eh)
{
	if (!eh->HasFd())
		return;

	int fd = eh->GetFd();
	if (static_cast<size_t>(fd) >= GetMaxFds())
		return;

	// Use Rust implementation
	rust_socketengine_select_del_fd(fd);

	SocketEngine::DelFdRef(eh);

	FD_CLR(fd, &ReadSet);
	FD_CLR(fd, &WriteSet);
	FD_CLR(fd, &ErrSet);
	if (fd == MaxFD)
		--MaxFD;

	ServerInstance->Logs.Debug("SOCKET", "Remove file descriptor: {}", fd);
}

void SocketEngine::OnSetEvent(EventHandler* eh, int old_mask, int new_mask)
{
	int fd = eh->GetFd();
	int diff = old_mask ^ new_mask;

	// Use Rust implementation
	rust_socketengine_select_mod_fd(fd, old_mask, new_mask, static_cast<void*>(eh));

	if (diff & (FD_WANT_POLL_READ | FD_WANT_FAST_READ))
	{
		if (new_mask & (FD_WANT_POLL_READ | FD_WANT_FAST_READ))
			FD_SET(fd, &ReadSet);
		else
			FD_CLR(fd, &ReadSet);
	}
	if (diff & (FD_WANT_POLL_WRITE | FD_WANT_FAST_WRITE | FD_WANT_SINGLE_WRITE))
	{
		if (new_mask & (FD_WANT_POLL_WRITE | FD_WANT_FAST_WRITE | FD_WANT_SINGLE_WRITE))
			FD_SET(fd, &WriteSet);
		else
			FD_CLR(fd, &WriteSet);
	}
}

int SocketEngine::DispatchEvents()
{
	// Use Rust implementation to wait for events
	int sresult = rust_socketengine_select_wait(1, 0);
	ServerInstance->UpdateTime();

	// Get the max_fd from Rust
	MaxFD = rust_socketengine_select_get_max_fd();

	for (int i = 0, j = sresult; i <= MaxFD && j > 0; i++)
	{
		// Use Rust to check if this fd has events
		int events = rust_socketengine_select_has_events(i, &ReadSet, &WriteSet, &ErrSet);
		
		int has_read = events & 1;
		int has_write = events & 2;
		int has_error = events & 4;

		if (!(has_read || has_write || has_error))
			continue;

		--j;

		EventHandler* ev = GetRef(i);
		if (!ev)
			continue;

		if (has_error)
		{
			stats.ErrorEvents++;

			socklen_t codesize = sizeof(int);
			int errcode = 0;
			if (getsockopt(i, SOL_SOCKET, SO_ERROR, (char*)&errcode, &codesize) < 0)
				errcode = errno;

			ev->OnEventHandlerError(errcode);
			continue;
		}

		if (has_read)
		{
			ev->SetEventMask(ev->GetEventMask() & ~FD_READ_WILL_BLOCK);
			ev->OnEventHandlerRead();
			if (ev != GetRef(i))
				continue;
		}

		if (has_write)
		{
			int newmask = (ev->GetEventMask() & ~(FD_WRITE_WILL_BLOCK | FD_WANT_SINGLE_WRITE));
			SocketEngine::OnSetEvent(ev, ev->GetEventMask(), newmask);
			ev->SetEventMask(newmask);
			ev->OnEventHandlerWrite();
		}
	}

	return sresult;
}
