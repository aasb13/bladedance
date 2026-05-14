/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2020 Matt Schatz <genius3000@g3k.solutions>
 *   Copyright (C) 2017-2020, 2022-2023 Sadie Powell <sadie@witchery.services>
 *   Copyright (C) 2012 Robby <robby@chatbelgie.be>
 *   Copyright (C) 2012 ChrisTX <xpipe@hotmail.de>
 *   Copyright (C) 2009-2010 Daniel De Graaf <danieldg@inspircd.org>
 *   Copyright (C) 2009 Uli Schlachter <psychon@znc.in>
 *   Copyright (C) 2007 Robin Burchell <robin+git@viroteck.net>
 *   Copyright (C) 2003, 2006 Craig Edwards <brain@inspircd.org>
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
#include "dynamic.h"
#include <dlfcn.h>
#include "rust_module.h"

// Static error string for FFI
static std::string g_error_string;

// FFI functions called from Rust
extern "C" CoreExport void* dynamic_ffi_create_rust_module_wrapper(
    const void* vtable,
    void* rust_handle,
    const char* libname,
    size_t libname_length)
{
    if (!vtable || !rust_handle || !libname)
        return nullptr;

    const RustModuleVtable* vtable_ptr = static_cast<const RustModuleVtable*>(vtable);
    std::string name(libname, libname_length);
    return new RustModuleWrapper(vtable_ptr, rust_handle, name);
}

extern "C" CoreExport void dynamic_ffi_free_error_string(char* ptr)
{
    if (ptr)
        delete[] ptr;
} 

extern "C" CoreExport char* dynamic_ffi_format_version_error(
    const char* libname,
    size_t libname_length,
    const char* version,
    unsigned long abi,
    unsigned long module_abi)
{
    std::string name(libname, libname_length);
    std::string ver_str(version ? version : "an unknown version");
    
    std::string error = INSP_FORMAT("{} was built against {} ({}) which is too {} to use with {} ({}).",
        name, ver_str, abi,
        abi < module_abi ? "old" : "new", INSPIRCD_VERSION, module_abi);
    
    char* result = new char[error.length() + 1];
    std::strcpy(result, error.c_str());
    return result;
}

extern "C" CoreExport void dynamic_ffi_set_error_string(char* ptr)
{
    if (ptr)
    {
        g_error_string = ptr;
        delete[] ptr;
    }
}

// DLLManager implementation using Rust FFI
DLLManager::DLLManager(const std::string& name)
    : libname(name)
    , rust_handle(nullptr)
{
    rust_handle = DLLManager_Create(name.c_str(), name.length());
    if (rust_handle)
    {
        char* error = DLLManager_LastError(rust_handle);
        if (error && *error)
        {
            err = error;
            DLLManager_FreeString(error);
        }
    }
    else
    {
        err = "Failed to create Rust DLLManager";
    }
}

DLLManager::~DLLManager()
{
    if (rust_handle)
    {
        DLLManager_Destroy(rust_handle);
        rust_handle = nullptr;
    }
}

Module* DLLManager::CallInit()
{
    if (!rust_handle)
        return nullptr;

    void* module_ptr = DLLManager_CallInit(rust_handle);
    if (!module_ptr)
    {
        char* error = DLLManager_LastError(rust_handle);
        if (error)
        {
            err = error;
            DLLManager_FreeString(error);
        }
    }
    return static_cast<Module*>(module_ptr);
}

void* DLLManager::GetSymbol(const char* name) const
{
    if (!rust_handle || !name)
        return nullptr;

    return const_cast<void*>(DLLManager_GetSymbol(rust_handle, name));
}
