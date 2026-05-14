#include "inspircd.h"
#include "rust_module.h"

RustModuleWrapper::RustModuleWrapper(const RustModuleVtable* vtable_ptr, void* raw_handle, const std::string& name)
    : Module(VF_VENDOR, "Rust Module")
    , rust_handle(raw_handle)
    , vtable(vtable_ptr)
{
    ModuleFile = name;
}

RustModuleWrapper::~RustModuleWrapper() {
    if (vtable && vtable->destroy)
        vtable->destroy(rust_handle);
}

void RustModuleWrapper::init() {
    // printf("RustModuleWrapper::init() called\n");
    // fflush(stdout);
    if (vtable && vtable->init)
        vtable->init(rust_handle);
}

void RustModuleWrapper::ReadConfig(ConfigStatus& status) {
    if (vtable && vtable->read_config)
        vtable->read_config(rust_handle);
}