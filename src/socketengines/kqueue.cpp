/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2021 Mistral AI
 *   Copyright (C) 2014-2015 Attila Molnar <attilamolnar@hush.com>
 *   Copyright (C) 2014 Adam <Adam@anope.org>
 *   Copyright (C) 2012, 2017, 2019, 2022-2023 Sadie Powell <sadie@witchery.services>
 *   Copyright (C) 2012 Robby <robby@chatbelgie.be>
 *   Copyright (C) 2009-2010 Daniel De Graaf <danieldg@inspircd.org>
 *   Copyright (C) 2009 Uli Schlachter <psychon@znc.in>
 *   Copyright (C) 2008 Thomas Stagner <aquanight@gmail.com>
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

#include <sys/types.h>
#include <sys/event.h>

/** Rust FFI declarations for kqueue socket engine */
extern "C" bool rust_socketengine_kqueue_init();
extern "C" void rust_socketengine_kqueue_deinit();
extern "C" bool rust_socketengine_kqueue_recover_from_fork();
extern "C" bool rust_socketengine_kqueue_add_fd(int fd, int event_mask, void* eh_ptr);
extern "C" bool rust_socketengine_kqueue_mod_fd(int fd, int old_mask, int new_mask, void* eh_ptr);
extern "C" bool rust_socketengine_kqueue_del_fd(int fd, void* eh_ptr);
extern "C" int rust_socketengine_kqueue_dispatch_events();
extern "C" int rust_socketengine_kqueue_get_event(size_t index, struct kevent* kev_out);

/** A specialisation of the SocketEngine class, designed to use BSD kqueue().
 */
namespace
{
#if defined __NetBSD__ && __NetBSD_Version__ <= 999001400
	inline intptr_t udata_cast(EventHandler* eh)
	{
		// On NetBSD <10 the last parameter of EV_SET is intptr_t.
		return reinterpret_cast<intptr_t>(eh);
	}
#else
	inline void* udata_cast(EventHandler* eh)
	{
		// On other platforms the last parameter of EV_SET is void*.
		return static_cast<void*>(eh);
	}
#endif
}

/** Initialize the kqueue engine
 */
void SocketEngine::Init()
{
	LookupMaxFds();
	
	// Initialize Rust kqueue engine
	if (!rust_socketengine_kqueue_init())
		InitError();
	
	RecoverFromFork();
}

void SocketEngine::RecoverFromFork()
{
	/*
	 * The only bad thing about kqueue is that its fd cant survive a fork and is not inherited.
	 * BUM HATS.
	 *
	 */
	// Use Rust to recover from fork
	if (!rust_socketengine_kqueue_recover_from_fork())
		InitError();
}

/** Shutdown the kqueue engine
 */
void SocketEngine::Deinit()
{
	// Clean up Rust kqueue engine
	rust_socketengine_kqueue_deinit();
}

bool SocketEngine::AddFd(EventHandler* eh, int event_mask)
{
	if (!eh->HasFd())
		return false;

	if (!SocketEngine::AddFdRef(eh))
		return false;

	// We always want to read from the socket...
	int fd = eh->GetFd();
	
	// Use Rust implementation
	if (!rust_socketengine_kqueue_add_fd(fd, event_mask, udata_cast(eh)))
		return false;

	::Logs.Debug("SOCKET", "New file descriptor: {}", fd);

	eh->SetEventMask(event_mask);
	OnSetEvent(eh, 0, event_mask);

	return true;
}

void SocketEngine::DelFd(EventHandler* eh)
{
	int fd = eh->GetFd();
	if (!eh->HasFd())
	{
		::Logs.Debug("SOCKET", "DelFd() on invalid fd: {}", fd);
		return;
	}

	// Use Rust implementation
	rust_socketengine_kqueue_del_fd(fd, udata_cast(eh));

	SocketEngine::DelFdRef(eh);

	::Logs.Debug("SOCKET", "Remove file descriptor: {}", fd);
}

void SocketEngine::OnSetEvent(EventHandler* eh, int old_mask, int new_mask)
{
	// Delegate fully to Rust implementation
	rust_socketengine_kqueue_mod_fd(eh->GetFd(), old_mask, new_mask, udata_cast(eh));
}

int SocketEngine::DispatchEvents()
{
	// Use Rust implementation to dispatch events
	int i = rust_socketengine_kqueue_dispatch_events();
	ServerInstance->UpdateTime();

	if (i < 0)
		return i;

	stats.TotalEvents += i;

	for (int j = 0; j < i; j++)
	{
		// Get event from Rust
		struct kevent kev;
		if (rust_socketengine_kqueue_get_event(j, &kev) != 0)
			continue;
	
		// This can't be a static_cast because udata is intptr_t on NetBSD.
		EventHandler* eh = reinterpret_cast<EventHandler*>(kev.udata);
		if (!eh || !eh->HasFd())
			continue;

		// Copy this in case the vector gets resized and kev invalidated
		const short filter = kev.filter;

		if (kev.flags & EV_EOF)
		{
			stats.ErrorEvents++;
			eh->OnEventHandlerError(kev.fflags);
			continue;
		}
		if (filter == EVFILT_WRITE)
		{
			/* When mask is FD_WANT_FAST_WRITE or FD_WANT_SINGLE_WRITE,
			 * we set a one-shot write, so we need to clear that bit
			 * to detect when it set again.
			 */
			const int bits_to_clr = FD_WANT_SINGLE_WRITE | FD_WANT_FAST_WRITE | FD_WRITE_WILL_BLOCK;
			eh->SetEventMask(eh->GetEventMask() & ~bits_to_clr);
			eh->OnEventHandlerWrite();
		}
		else if (filter == EVFILT_READ)
		{
			eh->SetEventMask(eh->GetEventMask() & ~FD_READ_WILL_BLOCK);
			eh->OnEventHandlerRead();
		}
	}

	return i;
}
