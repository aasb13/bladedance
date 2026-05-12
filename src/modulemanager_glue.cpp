/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2013, 2015, 2019-2024, 2026 Sadie Powell <sadie@witchery.services>
 *   Copyright (C) 2013 Adam <Adam@anope.org>
 *   Copyright (C) 2012-2013, 2015 Attila Molnar <attilamolnar@hush.com>
 *   Copyright (C) 2012 Robby <robby@chatbelgie.be>
 *   Copyright (C) 2009-2010 Daniel De Graaf <danieldg@inspircd.org>
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

#include <filesystem>

#include <fmt/color.h>

#include "inspircd.h"
#include "dynamic.h"

// Rust function declarations
extern "C" {
    bool rust_module_name_contains_path(const char* name);
    struct StdString;
    StdString rust_expand_mod_name(const char* modname, size_t modname_length);
    StdString rust_validate_module_file(const char* module_file_path, size_t module_file_path_length, const char* filename, size_t filename_length);
}

// C++ wrapper for the Rust ExpandModName function
std::string ModuleManager::ExpandModName(const std::string& modname)
{
    StdString result = rust_expand_mod_name(modname.c_str(), modname.length());
    
    if (result.data) {
        return std::string(result.data, result.length);
    }
    return "";
}

// This demonstrates the pattern: move logic to Rust, keep C++ glue thin
bool ModuleManager::Load(const std::string& modname, bool defer)
{
    // Example: Path validation logic moved to Rust
    // Previously this was done entirely in C++
    if (rust_module_name_contains_path(modname.c_str()))
    {
        LastModuleError = "You can't load modules with a path: " + modname;
        return false;
    }

    // The rest of the logic would be gradually moved to Rust
    // For now, we keep it in C++ to demonstrate the incremental approach
    const std::string filename = ExpandModName(modname);
    const std::string moduleFile = ServerInstance->Config->Paths.PrependModule(filename);

    // Use Rust validation function instead of C++ filesystem check
    StdString validation_result = rust_validate_module_file(moduleFile.c_str(), moduleFile.length(), filename.c_str(), filename.length());
    if (validation_result.data && validation_result.length > 0)
    {
        LastModuleError = std::string(validation_result.data, validation_result.length);
        ServerInstance->Logs.Critical("MODULE", LastModuleError);
        return false;
    }

    if (Modules.find(filename) != Modules.end())
    {
        LastModuleError = "Module " + filename + " is already loaded, cannot load a module twice!";
        ServerInstance->Logs.Critical("MODULE", LastModuleError);
        return false;
    }

    Module* newmod = nullptr;
    auto* newhandle = new DLLManager(moduleFile);
    ServiceList newservices;
    if (!defer)
        this->NewServices = &newservices;

    try
    {
        newmod = newhandle->CallInit();
        this->NewServices = nullptr;

        if (newmod)
        {
            newmod->ModuleFile = filename;
            newmod->ModuleDLL = newhandle;
            Modules[filename] = newmod;

            if (!defer)
            {
                AttachAll(newmod);
                AddServices(newservices);

                ConfigStatus confstatus;
                newmod->init();
                newmod->ReadConfig(confstatus);
            }

            ServerInstance->Logs.Normal("MODULE", "New module introduced: {} (version {}, properties {})",
                filename, newmod->GetVersion(), newmod->GetPropertyString());

            if (newmod->properties & VF_DEPRECATED)
            {
                ServerInstance->Logs.Warning("MODULE", "The {} module is deprecated and will be removed in the next version of InspIRCd!",
                    ModuleManager::ShrinkModName(filename));
            }
        }
        else
        {
            LastModuleError = "Unable to load " + filename + ": " + newhandle->LastError();
            ServerInstance->Logs.Critical("MODULE", LastModuleError);
            delete newhandle;
            return false;
        }
    }
    catch (const CoreException& modexcept)
    {
        this->NewServices = nullptr;

        // failure in module constructor
        if (newmod)
        {
            DoSafeUnload(newmod);
            ServerInstance->GlobalCulls.AddItem(newhandle);
        }
        else
            delete newhandle;
        LastModuleError = "Unable to load " + filename + ": " + modexcept.GetReason();
        ServerInstance->Logs.Critical("MODULE", LastModuleError);
        return false;
    }

    if (defer)
        return true;

    FOREACH_MOD(OnLoadModule, (newmod));
    PrioritizeHooks();
    return true;
}

// This demonstrates how LoadCoreModules would be gradually converted
void ModuleManager::LoadCoreModules(std::map<std::string, ServiceList>& servicemap)
{
    fmt::print("Loading core modules ");
    fflush(stdout);

    try
    {
        for (const auto& entry : std::filesystem::directory_iterator(ServerInstance->Config->Paths.Module))
        {
            if (!entry.is_regular_file())
                continue;

            const std::string name = entry.path().filename().string();
            if (!InspIRCd::Match(name, "core_*" DLL_EXTENSION))
                continue;

            fmt::print(".");
            fflush(stdout);
            this->NewServices = &servicemap[name];

            if (!Load(name, true))
            {
                fmt::println("");
                fmt::println("[{}] {}", fmt::styled("*", fmt::emphasis::bold | fmt::fg(fmt::terminal_color::red)), LastError());
                fmt::println("");
                ServerInstance->Exit(EXIT_FAILURE);
            }
        }
    }
    catch (const std::filesystem::filesystem_error& err)
    {
        fmt::println("failed: {}", err.what());
        ServerInstance->Exit(EXIT_FAILURE);
    }

    fmt::println("");
}
