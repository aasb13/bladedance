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

INSP_RUST_FFI_IMPL_BEGIN
extern "C" size_t channelmgr_ffi_max_channel_len()
{
	return ServerInstance->Config->Limits.MaxChannel;
}

extern "C" bool channelmgr_ffi_channels_is_prefix(unsigned char prefix)
{
	return ServerInstance->Channels.IsPrefix(prefix);
}
INSP_RUST_FFI_IMPL_END

Channel* ChannelManagerRustAccess::Find(const ChannelManager* cm, const std::string& channel)
{
	ChannelMap::const_iterator iter = cm->channels.find(channel);
	if (iter == cm->channels.end())
		return nullptr;

	return iter->second;
}
