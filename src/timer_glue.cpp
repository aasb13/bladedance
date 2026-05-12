/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2021-2023 Sadie Powell <sadie@witchery.services>
 *   Copyright (C) 2021 Dominic Hamon
 *   Copyright (C) 2013-2014 Attila Molnar <attilamolnar@hush.com>
 *   Copyright (C) 2012 Robby <robby@chatbelgie.be>
 *   Copyright (C) 2009 Uli Schlachter <psychon@znc.in>
 *   Copyright (C) 2009 Daniel De Graaf <danieldg@inspircd.org>
 *   Copyright (C) 2008 Robin Burchell <robin+git@viroteck.net>
 *   Copyright (C) 2007 Dennis Friis <peavey@inspircd.org>
 *   Copyright (C) 2007 Craig Edwards <brain@inspircd.org>
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

// Rust FFI declarations
extern "C" {
    uint64_t timer_rust_create_timer(uint64_t secs_from_now, bool repeating, void* cpp_timer);
    void timer_rust_destroy_timer(void* rust_timer);
    int64_t timer_rust_get_trigger(const void* rust_timer);
    void timer_rust_set_trigger(void* rust_timer, int64_t nexttrigger);
    uint64_t timer_rust_get_interval(const void* rust_timer);
    void timer_rust_set_interval(void* rust_timer, uint64_t newinterval, bool restart);
    bool timer_rust_get_repeat(const void* rust_timer);
    void timer_rust_cancel_repeat(void* rust_timer);
    void timer_rust_tick_timers();
    void timer_rust_add_timer(void* rust_timer);
    void timer_rust_del_timer(void* cpp_timer);
    int64_t timer_ffi_server_time();
}

// C++ to Rust bridge implementations
void Timer::SetInterval(unsigned long newinterval, bool restart)
{
    secs = newinterval;
    if (!restart)
        return;

    ServerInstance->Timers.DelTimer(this);
    SetTrigger(ServerInstance->Time() + newinterval);
    ServerInstance->Timers.AddTimer(this);
}

Timer::Timer(unsigned long secs_from_now, bool repeating)
    : secs(secs_from_now)
    , repeat(repeating)
{
    // Create Rust timer instance
    rust_timer = reinterpret_cast<void*>(timer_rust_create_timer(secs_from_now, repeating, this));
}

Timer::~Timer()
{
    if (GetTrigger())
        ServerInstance->Timers.DelTimer(this);
    
    // Destroy Rust timer instance
    if (rust_timer)
        timer_rust_destroy_timer(rust_timer);
}

void TimerManager::TickTimers()
{
    // Delegate to Rust implementation
    timer_rust_tick_timers();
}

void TimerManager::DelTimer(Timer* t)
{
    // Delegate to Rust implementation
    timer_rust_del_timer(t);
}

void TimerManager::AddTimer(Timer* t)
{
    // Delegate to Rust implementation
    timer_rust_add_timer(t->rust_timer);
}

// FFI functions for Rust to call C++ methods
extern "C" {
    __attribute__((visibility("default"))) int64_t timer_ffi_server_time()
    {
        return ServerInstance->Time();
    }
    
    __attribute__((visibility("default"))) void timer_ffi_add_timer(void* cpp_timer)
    {
        Timer* timer = static_cast<Timer*>(cpp_timer);
        if (timer)
        {
            // Use public AddTimer method instead of direct access
            ServerInstance->Timers.AddTimer(timer);
        }
    }
    
    __attribute__((visibility("default"))) void timer_ffi_del_timer(void* cpp_timer)
    {
        Timer* timer = static_cast<Timer*>(cpp_timer);
        if (timer)
        {
            // Use public DelTimer method instead of direct access
            ServerInstance->Timers.DelTimer(timer);
        }
    }
    
    __attribute__((visibility("default"))) bool timer_ffi_timer_tick(void* cpp_timer)
    {
        Timer* timer = static_cast<Timer*>(cpp_timer);
        if (timer)
            return timer->Tick();
        return false;
    }
    
    __attribute__((visibility("default"))) void* timer_ffi_get_rust_timer(void* cpp_timer)
    {
        Timer* timer = static_cast<Timer*>(cpp_timer);
        return timer ? timer->rust_timer : nullptr;
    }
}
