# rsync protocol notes

## Protocol negotiation

Protocol version is selected (in `exchange_version` and `setup_protocol`) as
the minimum of the protocols offered by the client and server, which makes
sense.

There is a separate text-mode greeting, including a protocol version, in
`exchange_protocols`, that sends a text string like `"@RSYNCD: %d.%d\n"`.  It
seems this is only used in the bare-TCP daemon (in `clientserver.c`) not over
SSH or locally. In `start_inband_exchange` the client sends authentication and
the module and args that it wants to use.

There's also a concept of "subprotocols", and comments indicate perhaps this is
for pre-release builds.  This might not be deployed widely enough to worry
about? This is also handled in `check_sub_protocol`. It basically seems to
downgrade to the prior protocol if the peer offers a subprotocol version that
the the local process doesn't support.

`compat.c` looks at the `client_info` string to both determine a protocol, and
to find some compatibility flags.  This only ever seems to get set from
`shell_cmd`, and that in turn seems to only come from the `--rsh` command line
option.  I don't understand how it could end up with the values this seems to
expect, unless perhaps it's passed as a hack in the daemon protocol, without
really representing the rsh command?

I guess this is set for daemon connections from arguments constructed in
`server_options`. 

## varint encoding 

The openrsync docs say that a 8-byte long is preceded by a maximum integer, but
it's actually preceded by `(int32) 0xffff_ffff`, in other words -1. (See
`read_longint`.

In addition to this encoding, there's also `read_varlong` which seems to read a
genuinely-variable length encoding.
