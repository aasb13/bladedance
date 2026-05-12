/*
 * InspIRCd -- Internet Relay Chat Daemon
 *
 *   Copyright (C) 2020-2023 Sadie Powell <sadie@witchery.services>
 */

#include "inspircd.h"
#include "utility/iterator_range.h"

extern "C" bool rust_channelmanager_default_is_channel(const char* data, size_t len);

bool ChannelManager::DefaultIsChannel(const std::string_view& channel)
{
	return rust_channelmanager_default_is_channel(channel.data(), channel.size());
}

Channel* ChannelManager::Find(const std::string& channel) const
{
	return ChannelManagerRustAccess::Find(this, channel);
}

bool ChannelManager::IsPrefix(unsigned char prefix) const
{
	// TODO: implement support for multiple channel types.
	return prefix == '#';
}
