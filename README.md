Tested on Arch Linux and Windows 10. Requires libnotify0.4-cil and Mono on Linux.

The purpose of this program is to display feeds from Broadcastify that suddenly spike in listeners or get updated with a status, as they can potentially serve as early news of large events taking place. 

# Usage

```
USAGE: BCInfo.exe [--help] [--threshold <percentage>] [--updatetime <minutes>] [--sort <ascending|descending>]

OPTIONS:

    --threshold, -p <percentage>
                          specify the percentage jump required to show a feed
    --updatetime, -t <minutes>
                          specify the minutes between updates (must be >= 5)
    --sort, -s <ascending|descending>
                          specify the order feeds will be displayed in
    --help                display this list of options.
```

The threshold argument specifies how sensitive the program will be with sudden jumps in listeners.
For example: if you used a threshold of 30 and there's a feed that averages 200 listeners, the feed will show up in the next update if it jumped to 260 listeners.

If you don't provide any arguments, the program will use 30% as the threshold and 6 minutes as the update time, which should be good enough for most users.

# Configuration

You can blacklist specific feeds or give them specific thresholds by modifying the *Config.yaml* file.
For example, to have the *Chicago Police* feed display if it jumps by 20% and the *Sacramento County Sheriff and Sacramento City Police*
feed display if it jumps by 10%, you could modify your config file to look like this:
```
---
Thresholds:
    - Name: Chicago Police
      Threshold: 20

    - Name: Sacramento County Sheriff and Sacramento City Police
      Threshold: 10
```

If you'd like to blacklist a feed from ever displaying, you can modify your *Config.yaml* file to look like this:
```
---
Blacklist:
    - Feed Name One
    - Feed Name Two
    - Feed Name Three
```

You can also blacklist and whitelist certain words for info messages that feeds will typical have when an event is taking place.
For example, if you only want to see feeds with info that contain the word "shooter", you can modify your config like this:
```
Info Whitelist:
    - shooter
```

If you don't wish to see feeds with info that contain the word "structure fire", you can modify your config like so:
```
Info Blacklist:
    - "structure fire"
```

Keep in mind that the terms you use are checked to see if they appear **anywhere** in the feed's info, so if you would like to blacklist / whitelist words such as "mva", it may be a good idea to surround it in spaces (ex. " mva ") so other words can't accidently trigger it.

A typical config file may look something like this:
```
---
Thresholds:
    - Name: Chicago Police
      Threshold: 20

    - Name: Sacramento County Sheriff and Sacramento City Police
      Threshold: 10

Blacklist:
    - Feed Name One
    - Feed Name Two
    - Feed Name Three

Info Whitelist:
    - Word One
    - Word Two
    - Word Three

Info Blacklist:
    - Word One
    - Word Two
    - Word Three
```

Also note that the config file is reloaded with every update, so changes will take effect immediately.