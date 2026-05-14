#include "inspircd.h"
#include "ctables.h"
#include "command_parse.h"

// Function pointer type for command handlers
typedef CmdResult (*CommandHandlerFunc)(User* user, const CommandBase::Params& parameters);

// Custom Command class that can have its handler set from Rust
class RustCommand : public Command
{
public:
    CommandHandlerFunc rust_handler;
    
    RustCommand(Module* me, const std::string& cmd, unsigned int minpara = 0, unsigned int maxpara = 0)
        : Command(me, cmd, minpara, maxpara), rust_handler(nullptr)
    {
    }
    
    CmdResult Handle(User* user, const Params& parameters) override
    {
        if (rust_handler)
            return rust_handler(user, parameters);
        return CmdResult::FAILURE;
    }
};

extern "C" {
    // Create a new Command instance
    Command* Command_Create(Module* module, const char* name, unsigned int min_params, unsigned int max_params);
    
    // Set the handler function for a Command
    void Command_SetHandler(Command* command, CommandHandlerFunc handler);
    
    // Add a command to the CommandParser
    bool CommandParser_AddCommand(Command* command);
}

// Create a new Command instance
Command* Command_Create(Module* module, const char* name, unsigned int min_params, unsigned int max_params)
{
    return new RustCommand(module, std::string(name), min_params, max_params);
}

// Set the handler function for a Command
void Command_SetHandler(Command* command, CommandHandlerFunc handler)
{
    RustCommand* rust_cmd = static_cast<RustCommand*>(command);
    if (rust_cmd)
        rust_cmd->rust_handler = handler;
}

// Add a command to the CommandParser
bool CommandParser_AddCommand(Command* command)
{
    return ServerInstance->Parser.AddCommand(command);
}
