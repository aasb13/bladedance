/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2025 Gateway Control Module
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
#include <mongoc/mongoc.h>
#include <bson/bson.h>
#include <dlfcn.h>
 
__attribute__((constructor))
static void force_load_mongoc() {
	dlopen("libmongoc-1.0.so", RTLD_NOW | RTLD_GLOBAL);
	dlopen("libbson-1.0.so", RTLD_NOW | RTLD_GLOBAL);
}
class CommandWhoami final
	: public Command
{
public:
	CommandWhoami(Module* Creator)
		: Command(Creator, "WHOAMI")
	{
	}

	CmdResult Handle(User* user, const Params& parameters) override
	{
		user->WriteRemoteNotice("Response");
		return CmdResult::SUCCESS;
	}
};

class ModuleGateway final
	: public Module
{
private:
	CommandWhoami cmd;
	bool db_initializing = false;
	bool db_initialized = false;
	mongoc_client_t* client = nullptr;
    mongoc_database_t* database = nullptr;
    mongoc_collection_t* collection = nullptr;

public:
	ModuleGateway()
		: Module(VF_VENDOR, "Gateway Control Module")
		, cmd(this)
	{
	}

	void ReadConfig(ConfigStatus& status) override
	{
		const auto& conf = ServerInstance->Config->ConfValue("gateway");
        std::string mongo_uri = conf->getString("mongouri", "mongodb://127.0.0.1:27017");

		if(!db_initialized && !db_initializing) {
			InitializeDatabase(mongo_uri);
		}
	}

	int InitializeDatabase(std::string uri)
	{
		int ret = _InitializeDatabase(uri);
		db_initializing = false;
		return ret;
	}

	int _InitializeDatabase(std::string uri)
	{
		db_initializing = true;

		mongoc_init();

		client = mongoc_client_new(uri.c_str());
        if (!client)
        {
            ServerInstance->Logs.Critical("m_gateway", "MongoDB connection failed");
            return -1;
        }

		database = mongoc_client_get_database(client, "irc");
        collection = mongoc_client_get_collection(client, "irc", "users");
		
		ServerInstance->Logs.Normal("m_gateway", "Database tables initialized");

		db_initialized = true;

		return 0;
	}

	int Clean()
	{
		if (collection) mongoc_collection_destroy(collection);
        if (database) mongoc_database_destroy(database);
        if (client) mongoc_client_destroy(client);
        mongoc_cleanup();

		return 0;
	}

	~ModuleGateway()
    {
        Clean();
    }
};

MODULE_INIT(ModuleGateway)