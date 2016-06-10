Tested on Arch Linux and Windows 10. Requires libnotify0.4-cil and Mono on Linux.

The purpose of this program is to display feeds from Broadcastify that suddenly spike in listeners or get updated with a status, as they can potentially serve as early news of large events taking place. 

# Usage

```
./BCInfo.exe <percentage jump to display feed> <update time in minutes>
```

The first argument specifies how sensitive the program will be with sudden jumps in listeners. For example, if you used a percentage of 30%, the program would only display feeds that went from 100 average listeners to 130 listeners by the next update.

If you don't provide any arguments, the program will use 30% as the percentage and 6 minutes as the update time, which should be good enough for most users.

You can also specify different percentages for specific feeds by creating a file next to the executable named *thresholds.csv* and putting the feed name and percentage in it separated by a comma.
Note that if the feed name contains any commas, you'll have to wrap the entire feed name in quotes. Here's an example which contains two feeds:
```
"Folsom, Citrus Heights, Elk Grove, and West Sacramento Police",15
Sacramento County Sheriff and Sacramento City Police,20
```

Note that the *thresholds.csv* file is loaded with every update so you can observe changes quickly.