#ifndef RUST_MODULE_H
#define RUST_MODULE_H

#include "inspircd.h"

struct RustModuleVtable {
    void (*init)(void*);
    void (*read_config)(void*);
    void (*destroy)(void*);
};

class RustModuleWrapper : public Module {
    void* rust_handle;
    const RustModuleVtable* vtable;
public:
    RustModuleWrapper(const RustModuleVtable* vtable_ptr, void* raw_handle, const std::string& name);
    ~RustModuleWrapper();
    void init() override;
    void ReadConfig(ConfigStatus& status) override;
};

#endif
