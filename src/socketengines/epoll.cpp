/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2021 Mistral AI
 *   Copyright (C) 2017, 2019, 2022-2023 Sadie Powell <sadie@witchery.services>
 *   Copyright (C) 2014-2015 Attila Molnar <attilamolnar@hush.com>
 *   Copyright (C) 2014, 2016 Adam <Adam@anope.org>
 *   Copyright (C) 2012 Robby <robby@chatbelgie.be>
 *   Copyright (C) 2012 Ariadne Conill <ariadne@dereferenced.org>
 *   Copyright (C) 2009-2010 Daniel De Graaf <danieldg@inspircd.org>
 *   Copyright (C) 2008 Thomas Stagner <aquanight@gmail.com>
 *   Copyright (C) 2007-2008 Dennis Friis <peavey@inspircd.org>
 *   Copyright (C) 2006-2008 Craig Edwards <brain@inspircd.org>
 *   Copyright (C) 2006, 2008 Robin Burchell <robin+git@viroteck.net>
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

#include <sys/epoll.h>
#include <sys/resource.h>

/** Rust FFI declarations for epoll socket engine */
extern "C" bool rust_socketengine_epoll_init();
extern "C" void rust_socketengine_epoll_deinit();
extern "C" bool rust_socketengine_epoll_recover_from_fork();
extern "C" bool rust_socketengine_epoll_add_fd(int fd, int event_mask, void* eh_ptr);
extern "C" bool rust_socketengine_epoll_mod_fd(int fd, int event_mask, void* eh_ptr);
extern "C" bool rust_socketengine_epoll_del_fd(int fd);
extern "C" int rust_socketengine_epoll_wait(struct epoll_event* events, int max_events, int timeout_ms);

/** These are used by epoll() to hold socket events
 */
namespace
{
	std::vector<struct epoll_event> events(16);
}

void SocketEngine::Init()
{
	LookupMaxFds();

	// Initialize Rust epoll engine
	if (!rust_socketengine_epoll_init())
		InitError();
}

void SocketEngine::RecoverFromFork()
{
	// Recover Rust epoll engine after fork
	rust_socketengine_epoll_recover_from_fork();
}

void SocketEngine::Deinit()
{
	// Clean up Rust epoll engine
	rust_socketengine_epoll_deinit();
}

static unsigned mask_to_epoll(int event_mask)
{
	unsigned rv = 0;
	if (event_mask & (FD_WANT_POLL_READ | FD_WANT_POLL_WRITE | FD_WANT_SINGLE_WRITE))
	{
		// we need to use standard polling on this FD
		if (event_mask & (FD_WANT_POLL_READ | FD_WANT_FAST_READ))
			rv |= EPOLLIN;
		if (event_mask & (FD_WANT_POLL_WRITE | FD_WANT_FAST_WRITE | FD_WANT_SINGLE_WRITE))
			rv |= EPOLLOUT;
	}
	else
	{
		// we can use edge-triggered polling on this FD
		rv = EPOLLET;
		if (event_mask & (FD_WANT_FAST_READ | FD_WANT_EDGE_READ))
			rv |= EPOLLIN;
		if (event_mask & (FD_WANT_FAST_WRITE | FD_WANT_EDGE_WRITE))
			rv |= EPOLLOUT;
	}
	return rv;
}

bool SocketEngine::AddFd(EventHandler* eh, int event_mask)
{
	int fd = eh->GetFd();
	if (!eh->HasFd())
	{
		::Logs.Debug("SOCKET", "AddFd out of range: (fd: {})", fd);
		return false;
	}

	if (!SocketEngine::AddFdRef(eh))
	{
		::Logs.Debug("SOCKET", "Attempt to add duplicate fd: {}", fd);
		return false;
	}

	// Use Rust implementation
	if (!rust_socketengine_epoll_add_fd(fd, event_mask, static_cast<void*>(eh)))
	{
		::Logs.Debug("SOCKET", "Error adding fd: {} to socketengine: {}", fd, strerror(errno));
		return false;
	}

	::Logs.Debug("SOCKET", "New file descriptor: {}", fd);

	eh->SetEventMask(event_mask);
	ResizeDouble(events);

	return true;
}

void SocketEngine::OnSetEvent(EventHandler* eh, int old_mask, int new_mask)
{
	unsigned old_events = mask_to_epoll(old_mask);
	unsigned new_events = mask_to_epoll(new_mask);
	if (old_events != new_events)
	{
		// Use Rust implementation to modify the event
		rust_socketengine_epoll_mod_fd(eh->GetFd(), new_mask, static_cast<void*>(eh));
	}
}

void SocketEngine::DelFd(EventHandler* eh)
{
	int fd = eh->GetFd();
	if (!eh->HasFd())
	{
		::Logs.Debug("SOCKET", "DelFd out of range: (fd: {})", fd);
		return;
	}

	// Use Rust implementation to remove the fd
	rust_socketengine_epoll_del_fd(fd);

	SocketEngine::DelFdRef(eh);

	::Logs.Debug("SOCKET", "Remove file descriptor: {}", fd);
}

int SocketEngine::DispatchEvents()
{
	// Use Rust implementation to wait for events
	int i = rust_socketengine_epoll_wait(events.data(), static_cast<int>(events.size()), 1000);
	ServerInstance->UpdateTime();

	stats.TotalEvents += i;

	for (int j = 0; j < i; j++)
	{
		// Copy these in case the vector gets resized and ev invalidated
		const epoll_event ev = events[j];

		EventHandler* const eh = static_cast<EventHandler*>(ev.data.ptr);
		if (!eh->HasFd())
			continue;

		const int fd = eh->GetFd();
		if (ev.events & EPOLLHUP)
		{
			stats.ErrorEvents++;
			eh->OnEventHandlerError(0);
			continue;
		}

		if (ev.events & EPOLLERR)
		{
			stats.ErrorEvents++;
			/* Get error number */
			socklen_t codesize = sizeof(int);
			int errcode;
			if (getsockopt(fd, SOL_SOCKET, SO_ERROR, &errcode, &codesize) < 0)
				errcode = errno;
			eh->OnEventHandlerError(errcode);
			continue;
		}

		int mask = eh->GetEventMask();
		if (ev.events & EPOLLIN)
			mask &= ~FD_READ_WILL_BLOCK;
		if (ev.events & EPOLLOUT)
		{
			mask &= ~FD_WRITE_WILL_BLOCK;
			if (mask & FD_WANT_SINGLE_WRITE)
			{
				int nm = mask & ~FD_WANT_SINGLE_WRITE;
				OnSetEvent(eh, mask, nm);
				mask = nm;
			}
		}
		eh->SetEventMask(mask);
		if (ev.events & EPOLLIN)
		{
			eh->OnEventHandlerRead();
			if (eh != GetRef(fd))
				// whoa! we got deleted, better not give out the write event
				continue;
		}
		if (ev.events & EPOLLOUT)
		{
			eh->OnEventHandlerWrite();
		}
	}

	return i;
}
