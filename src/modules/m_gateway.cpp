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
#include "modules/account.h"

__attribute__((constructor)) static void force_load_mongoc()
{
	dlopen("libmongoc-1.0.so", RTLD_NOW | RTLD_GLOBAL);
	dlopen("libbson-1.0.so", RTLD_NOW | RTLD_GLOBAL);
}

class ModuleGateway final : public Module
{
private:
	Account::API accountapi;
	bool db_initializing = false;
	bool db_initialized = false;
	mongoc_client_t *client = nullptr;
	mongoc_database_t *database = nullptr;
	mongoc_collection_t *users_collection = nullptr;
	mongoc_collection_t *channels_collection = nullptr;

	class CommandWhoami final : public Command
	{
		ModuleGateway *mod;

	public:
		CommandWhoami(Module *Creator, ModuleGateway *m)
			: Command(Creator, "WHOAMI"),
			  mod(m)
		{
		}

		CmdResult Handle(User *user, const Params &parameters) override
		{
			int level = GetUserLevel(user);
			std::string *account = mod->accountapi->GetAccountName(user);
			std::string accountName = (account && !account->empty()) ? *account : "none";

			user->WriteRemoteNotice("Account: " + accountName + " | Level: " + ConvToStr(level));
			return CmdResult::SUCCESS;
		}
	};
	CommandWhoami cmd;

public:
	ModuleGateway()
		: Module(VF_VENDOR, "Gateway Control Module"), accountapi(this), cmd(this, this)
	{
	}

	void ReadConfig(ConfigStatus &status) override
	{
		const auto &conf = ServerInstance->Config->ConfValue("gateway");
		std::string mongo_uri = conf->getString("mongouri", "mongodb://127.0.0.1:27017");

		if (!db_initialized && !db_initializing)
		{
			InitializeDatabase(mongo_uri);
		}
	}

	int _GetUserLevel(User *user)
	{
		if (!client || !users_collection)
			return 0;

		if (user->IsOper())
			return 4;

		// Build query: find user by account name or nick
		std::string *account = accountapi->GetAccountName(user);
		if (!account || account->empty())
			return 0;

		bson_t *query = BCON_NEW("account_name", BCON_UTF8(account->c_str()));

		bson_t *opts = BCON_NEW("projection", "{", "userlevel", BCON_INT32(1), "}");

		mongoc_cursor_t *cursor = mongoc_collection_find_with_opts(users_collection, query, opts, NULL);

		const bson_t *doc;
		int level = 0;
		if (mongoc_cursor_next(cursor, &doc))
		{
			bson_iter_t iter;
			if (bson_iter_init_find(&iter, doc, "userlevel"))
				level = bson_iter_int32(&iter);
		}

		mongoc_cursor_destroy(cursor);
		bson_destroy(query);
		bson_destroy(opts);

		return level;
	}

	void SetUserLevel(const std::string &account_name, int level)
	{
		if (!client || !users_collection)
			return;

		bson_t *filter = BCON_NEW("account_name", BCON_UTF8(account_name.c_str()));
		bson_t *update = BCON_NEW("$set", "{", "userlevel", BCON_INT32(level), "}");

		mongoc_collection_update_one(users_collection, filter, update, NULL, NULL, NULL);

		bson_destroy(filter);
		bson_destroy(update);
	}

	void AddUser(const std::string &account_name, int level)
	{
		if (!client || !users_collection)
			return;

		bson_t *doc = BCON_NEW(
			"account_name", BCON_UTF8(account_name.c_str()),
			"userlevel", BCON_INT32(level),
			"created_at", BCON_DATE_TIME(time(NULL) * 1000));

		bson_error_t error;
		if (!mongoc_collection_insert_one(users_collection, doc, NULL, NULL, &error))
			ServerInstance->Logs.Warning("m_gateway", "Failed to add user: {}", error.message);

		bson_destroy(doc);
	}

	void init() override
	{
		GetUserLevel = [this](User *user) -> int
		{
			return _GetUserLevel(user);
		};
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
		ServerInstance->Logs.Critical("m_gateway", "MongoDB connected");

		database = mongoc_client_get_database(client, "irc");
		users_collection = mongoc_client_get_collection(client, "irc", "users");
		channels_collection = mongoc_client_get_collection(client, "irc", "channels");

		ServerInstance->Logs.Normal("m_gateway", "Database initialized");

		db_initialized = true;

		return 0;
	}

	int Clean()
	{
		if (users_collection)
			mongoc_collection_destroy(users_collection);
		if (channels_collection)
			mongoc_collection_destroy(channels_collection);
		if (database)
			mongoc_database_destroy(database);
		if (client)
			mongoc_client_destroy(client);
		mongoc_cleanup();

		return 0;
	}

	~ModuleGateway()
	{
		Clean();
	}
};

MODULE_INIT(ModuleGateway)