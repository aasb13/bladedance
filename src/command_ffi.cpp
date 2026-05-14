#include "inspircd.h"
#include "ctables.h"
#include "command_parse.h"

typedef CmdResult (*CommandHandlerFunc)(User* user, const CommandBase::Params& parameters);

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
    Command* Command_Create(Module* module, const char* name, unsigned int min_params, unsigned int max_params);
    void Command_SetHandler(Command* command, CommandHandlerFunc handler);
    bool CommandParser_AddCommand(Command* command);
}

Command* Command_Create(Module* module, const char* name, unsigned int min_params, unsigned int max_params)
{
    return new RustCommand(module, std::string(name), min_params, max_params);
}

void Command_SetHandler(Command* command, CommandHandlerFunc handler)
{
    RustCommand* rust_cmd = static_cast<RustCommand*>(command);
    if (rust_cmd)
        rust_cmd->rust_handler = handler;
}

bool CommandParser_AddCommand(Command* command)
{
    return ServerInstance->Parser.AddCommand(command);
}
