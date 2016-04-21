Tested on Arch Linux and Windows 10. Requires libnotify0.4-cil and Mono on Linux.

The program works by comparing the current listeners of each feed to its average hourly listener count which is then multiplied by a user-defined percentage to filter out feeds that normally have listener counts that spike during different times of the day.

For example: If you launch the program with a percentage of 30, feeds that spike in listeners by 30% of its current hourly average will be displayed in the next update.

# Usage

```
./BCInfo.exe <percentage jump to display feed> <update time in minutes>
```

If you do not pass any arguments to the program, it will use 30% as the percentage and 6 minutes as the update time, so it will check for changes 10 times per hour.
