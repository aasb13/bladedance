/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2020-2023 Sadie Powell <sadie@witchery.services>
 */

#include "inspircd.h"

extern "C" size_t channelmgr_ffi_max_channel_len()
{
	return ServerInstance->Config->Limits.MaxChannel;
}

extern "C" bool channelmgr_ffi_channels_is_prefix(unsigned char prefix)
{
	return ServerInstance->Channels.IsPrefix(prefix);
}

Channel* ChannelManagerRustAccess::Find(const ChannelManager* cm, const std::string& channel)
{
	ChannelMap::const_iterator iter = cm->channels.find(channel);
	if (iter == cm->channels.end())
		return nullptr;

	return iter->second;
}
