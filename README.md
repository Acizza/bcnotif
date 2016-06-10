Tested on Arch Linux and Windows 10. Requires libnotify0.4-cil and Mono on Linux.

The purpose of this program is to display feeds from Broadcastify that suddenly spike in listeners or get updated with a status, as they can potentially serve as early news of large events taking place. 

# Usage

```
./BCInfo.exe <percentage jump to display feed> <update time in minutes>
```

The first argument specifies how sensitive the program will be with sudden jumps in listeners. For example, if you used a percentage of 30%, the program would only display feeds that went from 100 average listeners to 130 listeners by the next update.

If you don't provide any arguments, the program will use 30% as the percentage and 6 minutes as the update time, which should be good enough for most users.