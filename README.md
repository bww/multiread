# Multiread
This is mainly a learning project for my forays into Rust. I woudln't recommend using it for anything that matters just yet.

## What's this thing for?
When it actually works right, this will function as a command line tool for collecting input from any one of many terminals which are concurrently blocking on input from STDIN. Once input is received by any one of the terminals it will be forwarded to all the others so the same input does not need to be manually entered on each of them. One input, many recipients.

```
# Terminal 1
$ multiread -k some-key
> Enter some data: 
< Hello

# Terminal 2
$ multiread -k some-key
> Enter some data: Hello^D
< Hello
```

