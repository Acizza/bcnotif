Tested on Arch Linux and Windows 10. Requires libnotify0.4-cil and Mono on Linux.

The program works by comparing the current listeners of each feed to its average listeners multiplied by a user-defined percentage to eliminate feeds that normally have high listener counts.

For example, with a percentage of 30, and a feed that averages 100 listeners, the feed will only show up in the next update if the number of listeners is at or above 130.

# Usage

```
./BC-Info.exe <percentage jump to display feed> <update time in minutes>
```

If you do not pass any arguments to the program, it will use 30% as the percentage and 6 minutes as the update time, so it will check for changes 10 times per hour.
