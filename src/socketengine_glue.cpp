/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2021 Dominic Hamon
 *   Copyright (C) 2017-2018, 2021-2024 Sadie Powell <sadie@witchery.services>
 *   Copyright (C) 2017 Robin Burchell <robin+git@viroteck.net>
 *   Copyright (C) 2013-2014 Adam <Adam@anope.org>
 *   Copyright (C) 2012, 2014-2015 Attila Molnar <attilamolnar@hush.com>
 *   Copyright (C) 2012 Robby <robby@chatbelgie.be>
 *   Copyright (C) 2012 ChrisTX <xpipe@hotmail.de>
 *   Copyright (C) 2009 Daniel De Graaf <danieldg@inspircd.org>
 *   Copyright (C) 2007 burlex
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


#ifndef _WIN32
# include <fcntl.h>
# include <sys/resource.h>
# include <unistd.h>
#endif

#include <fmt/color.h>

#include "inspircd.h"

extern "C" int rust_socketengine_close(int fd);
extern "C" int rust_socketengine_blocking(int fd);
extern "C" int rust_socketengine_nonblocking(int fd);
extern "C" void rust_socketengine_stats_update_read_counters(ssize_t len_in, uint64_t* read_events, size_t* indata, uint64_t* error_events);
extern "C" void rust_socketengine_stats_update_write_counters(ssize_t len_out, uint64_t* write_events, size_t* outdata, uint64_t* error_events);
extern "C" void rust_socketengine_stats_get_bandwidth(size_t indata, size_t outdata, float* kbitpersec_in, float* kbitpersec_out, float* kbitpersec_total);
extern "C" int rust_socketengine_connect(int fd, const sockaddr* addr, socklen_t addrlen);
extern "C" int rust_socketengine_bind(int fd, const sockaddr* addr, socklen_t addrlen);
extern "C" int rust_socketengine_shutdown(int fd, int how);
extern "C" int rust_socketengine_listen(int fd, int backlog);
extern "C" int rust_socketengine_accept(int fd, sockaddr* addr, socklen_t* addrlen);
extern "C" ssize_t rust_socketengine_recvfrom(int fd, void* buf, size_t len, int flags, sockaddr* from, socklen_t* fromlen);
extern "C" ssize_t rust_socketengine_send(int fd, const void* buf, size_t len, int flags);
extern "C" ssize_t rust_socketengine_recv(int fd, void* buf, size_t len, int flags);
extern "C" ssize_t rust_socketengine_sendto(int fd, const void* buf, size_t len, int flags, const sockaddr* to, socklen_t tolen);
extern "C" int rust_socketengine_set_option(int fd, int level, int name, int value);

/** Reference table, contains all current handlers
 **/
std::vector<EventHandler*> SocketEngine::ref;

/** Current number of descriptors in the engine
 */
size_t SocketEngine::CurrentSetSize = 0;

/** List of handlers that want a trial read/write
 */
std::set<int> SocketEngine::trials;

size_t SocketEngine::MaxSetSize = 0;

/** Socket engine statistics: count of various events, bandwidth usage
 */
SocketEngine::Statistics SocketEngine::stats;

void EventHandler::SetFd(int FD)
{
	this->fd = FD;
}

void EventHandler::OnEventHandlerWrite()
{
}

void EventHandler::OnEventHandlerError(int errornum)
{
}

void SocketEngine::InitError()
{
	fmt::println(stderr, "{} Socket engine initialization failed. {}.", fmt::styled("FATAL ERROR!", fmt::emphasis::bold | fmt::fg(fmt::terminal_color::red)), strerror(errno));
	exit(EXIT_FAILURE);
}

void SocketEngine::LookupMaxFds()
{
#if defined _WIN32
	MaxSetSize = FD_SETSIZE;
#else
	struct rlimit limits;
	if (!getrlimit(RLIMIT_NOFILE, &limits))
		MaxSetSize = limits.rlim_cur;

#if defined __APPLE__
	limits.rlim_cur = limits.rlim_max == RLIM_INFINITY ? OPEN_MAX : limits.rlim_max;
#else
	limits.rlim_cur = limits.rlim_max;
#endif
	if (!setrlimit(RLIMIT_NOFILE, &limits))
		MaxSetSize = limits.rlim_cur;
#endif
}

void SocketEngine::ChangeEventMask(EventHandler* eh, int change)
{
	int old_m = eh->event_mask;
	int new_m = old_m;

	// if we are changing read/write type, remove the previously set bit
	if (change & FD_WANT_READ_MASK)
		new_m &= ~FD_WANT_READ_MASK;
	if (change & FD_WANT_WRITE_MASK)
		new_m &= ~FD_WANT_WRITE_MASK;

	// if adding a trial read/write, insert it into the set
	if (change & FD_TRIAL_NOTE_MASK && !(old_m & FD_TRIAL_NOTE_MASK))
		trials.insert(eh->GetFd());

	new_m |= change;
	if (new_m == old_m)
		return;

	eh->event_mask = new_m;
	OnSetEvent(eh, old_m, new_m);
}

void SocketEngine::DispatchTrialWrites()
{
	std::vector<int> working_list;
	working_list.reserve(trials.size());
	working_list.assign(trials.begin(), trials.end());
	trials.clear();
	for(int fd : working_list)
	{
		EventHandler* eh = GetRef(fd);
		if (!eh)
			continue;

		int mask = eh->event_mask;
		eh->event_mask &= ~(FD_ADD_TRIAL_READ | FD_ADD_TRIAL_WRITE);
		if ((mask & (FD_ADD_TRIAL_READ | FD_READ_WILL_BLOCK)) == FD_ADD_TRIAL_READ)
			eh->OnEventHandlerRead();
		if ((mask & (FD_ADD_TRIAL_WRITE | FD_WRITE_WILL_BLOCK)) == FD_ADD_TRIAL_WRITE)
			eh->OnEventHandlerWrite();
	}
}

bool SocketEngine::AddFdRef(EventHandler* eh)
{
	int fd = eh->GetFd();
	if (HasFd(fd))
		return false;

	while (static_cast<unsigned int>(fd) >= ref.size())
		ref.resize(ref.empty() ? 1 : (ref.size() * 2));
	ref[fd] = eh;
	CurrentSetSize++;
	return true;
}

void SocketEngine::DelFdRef(EventHandler* eh)
{
	int fd = eh->GetFd();
	if (GetRef(fd) == eh)
	{
		ref[fd] = nullptr;
		CurrentSetSize--;
	}
}

bool SocketEngine::HasFd(int fd)
{
	return GetRef(fd) != nullptr;
}

EventHandler* SocketEngine::GetRef(int fd)
{
	if (fd < 0 || static_cast<size_t>(fd) >= ref.size())
		return nullptr;
	return ref[fd];
}

int SocketEngine::Accept(EventHandler* eh, sockaddr* addr, socklen_t* addrlen)
{
	return rust_socketengine_accept(eh->GetFd(), addr, addrlen);
}

int SocketEngine::Close(EventHandler* eh)
{
	DelFd(eh);
	int ret = Close(eh->GetFd());
	eh->SetFd(-1);
	return ret;
}

int SocketEngine::Close(int fd)
{
	return rust_socketengine_close(fd);
}

int SocketEngine::Blocking(int fd)
{
	return rust_socketengine_blocking(fd);
}

int SocketEngine::NonBlocking(int fd)
{
	return rust_socketengine_nonblocking(fd);
}

ssize_t SocketEngine::RecvFrom(EventHandler* eh, void* buf, size_t len, int flags, sockaddr* from, socklen_t* fromlen)
{
	ssize_t nbRecvd = rust_socketengine_recvfrom(eh->GetFd(), buf, len, flags, from, fromlen);
	stats.UpdateReadCounters(nbRecvd);
	return nbRecvd;
}

ssize_t SocketEngine::Send(EventHandler* eh, const void* buf, size_t len, int flags)
{
	ssize_t nbSent = rust_socketengine_send(eh->GetFd(), buf, len, flags);
	stats.UpdateWriteCounters(nbSent);
	return nbSent;
}

ssize_t SocketEngine::Recv(EventHandler* eh, void* buf, size_t len, int flags)
{
	ssize_t nbRecvd = rust_socketengine_recv(eh->GetFd(), buf, len, flags);
	stats.UpdateReadCounters(nbRecvd);
	return nbRecvd;
}

ssize_t SocketEngine::SendTo(EventHandler* eh, const void* buf, size_t len, int flags, const irc::sockets::sockaddrs& address)
{
	ssize_t nbSent = rust_socketengine_sendto(eh->GetFd(), buf, len, flags, &address.sa, address.sa_size());
	stats.UpdateWriteCounters(nbSent);
	return nbSent;
}

ssize_t SocketEngine::WriteV(EventHandler* eh, const IOVector* iov, int count)
{
	ssize_t sent = writev(eh->GetFd(), iov, count);
	stats.UpdateWriteCounters(sent);
	return sent;
}

#ifdef _WIN32
int SocketEngine::WriteV(EventHandler* eh, const iovec* iovec, int count)
{
	// On Windows the fields in iovec are not in the order required by the Winsock API; IOVector has
	// the fields in the correct order.
	// Create temporary IOVectors from the iovecs and pass them to the WriteV() method that accepts the
	// platform's native struct.
	IOVector wiovec[128];
	count = std::min(count, static_cast<int>(sizeof(wiovec) / sizeof(IOVector)));

	for (int i = 0; i < count; i++)
	{
		wiovec[i].iov_len = iovec[i].iov_len;
		wiovec[i].iov_base = reinterpret_cast<char*>(iovec[i].iov_base);
	}
	return WriteV(eh, wiovec, count);
}
#endif

int SocketEngine::Connect(EventHandler* eh, const irc::sockets::sockaddrs& address)
{
	return rust_socketengine_connect(eh->GetFd(), &address.sa, address.sa_size());
}

int SocketEngine::Shutdown(EventHandler* eh, int how)
{
	return rust_socketengine_shutdown(eh->GetFd(), how);
}

int SocketEngine::Bind(EventHandler* eh, const irc::sockets::sockaddrs& addr)
{
	return rust_socketengine_bind(eh->GetFd(), &addr.sa, addr.sa_size());
}

int SocketEngine::Listen(EventHandler* eh, int backlog)
{
	return rust_socketengine_listen(eh->GetFd(), backlog);
}

void SocketEngine::Statistics::UpdateReadCounters(ssize_t len_in)
{
	CheckFlush();

	rust_socketengine_stats_update_read_counters(len_in, &ReadEvents, &indata, &ErrorEvents);
}

void SocketEngine::Statistics::UpdateWriteCounters(ssize_t len_out)
{
	CheckFlush();

	rust_socketengine_stats_update_write_counters(len_out, &WriteEvents, &outdata, &ErrorEvents);
}

void SocketEngine::Statistics::CheckFlush() const
{
	// Reset the in/out byte counters if it has been more than a second
	time_t now = ServerInstance->Time();
	if (lastempty != now)
	{
		lastempty = now;
		indata = outdata = 0;
	}
}

void SocketEngine::Statistics::GetBandwidth(float& kbitpersec_in, float& kbitpersec_out, float& kbitpersec_total) const
{
	CheckFlush();
	rust_socketengine_stats_get_bandwidth(indata, outdata, &kbitpersec_in, &kbitpersec_out, &kbitpersec_total);
}

std::string SocketEngine::LastError()
{
#ifndef _WIN32
	return strerror(errno);
#else
	std::string err = GetErrorMessage(WSAGetLastError());
	for (size_t pos = 0; ((pos = err.find_first_of("\r\n", pos)) != std::string::npos); )
		err[pos] = ' ';
	return err;
#endif
}

std::string SocketEngine::GetError(int errnum)
{
#ifndef _WIN32
	return strerror(errnum);
#else
	WSASetLastError(errnum);
	return LastError();
#endif
}

extern "C" int rust_socketengine_set_option(int fd, int level, int name, int value)
{
#ifndef _WIN32
	return setsockopt(fd, level, name, &value, sizeof(value));
#else
	// Windows setsockopt takes a const char* for the value parameter
	return setsockopt(fd, level, name, (const char*)&value, sizeof(value));
#endif
}
