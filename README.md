# bcnotif

The purpose of this program is to find audio feeds on Broadcastify that suddenly jump in listeners (or have an alert) and display a desktop notification for them, as a sudden jump in listeners usually means that some kind of event or emergency is happening. Since some people tune into feeds as soon as they hear multiple sirens, you can usually get notified of big incidents faster than you would following the news.

# Usage
This program runs in the background and uses a default configuration that should be suitable for most uses, so it can be launched directly without having to configure anything.

# Configuration
You can configure the application in many ways by creating a `config.yaml` file in the application directory.
Configuration options include adding a state to monitor, changing the update time, changing the order feeds are displayed in, and using different spike values (which determine if a feed has suddenly jumped in listeners) for different days of the week, or for a certain feed ID, state, county, or feed name.

For example, to process a certain state's feeds during an update, you can add this to the config file:
```yaml
Misc:
  State Feeds ID: 6 # California
```

Where `6` is the state's ID. Since there is no convenient way to find states by their name, you have to provide its ID instead. To get the ID, go [here](http://www.broadcastify.com/listen/), select the desired state, and use the number from the end of the URL.

Another common configuration would be increase the spike values for the weekend. An example of such configuration is as follows:
```yaml
Weekday Spike Percentages:
  - Friday:
    Jump Required: 0.4
  - Saturday:
    Jump Required: 0.4
  - Sunday:
    Jump Required: 0.4
```

The above configuration will increase the jump required for Friday, Saturday, and Sunday by 10% from the default, which will reduce the otherwise increased feed updates since more people have the opportunity to listen to feeds on the weekend.

Yet another common configuration would be to make feeds in your local area more sensitive to updates, like so:
```yaml
Feed Settings:
  - County: Sacramento
    Spike Percentages:
      Jump Required: 0.25
```

The above configuration will make feeds in Sacramento 5% more sensitive to feed fluctuations than normal. Note that there are other ways to specify what feeds you want to modify, and are explained at the example at the bottom of this section.

Naturally, you can also combine the above configurations to do something like process all feeds in California, make feeds in Sacramento more sensitive, and increase the jump required for the weekend, like so:
```yaml
Misc:
  State Feeds ID: 6 # California

Feed Settings:
  - County: Sacramento
    Spike Percentages:
      Jump Required: 0.25

Weekday Spike Percentages:
  - Friday:
    Jump Required: 0.4
  - Saturday:
    Jump Required: 0.4
  - Sunday:
    Jump Required: 0.4
```

Since explaining all of the configuration layouts would be complicated, here is an example showing all of the configuration options in use with their default values (also note that virtually all fields are optional):

```yaml
# Miscellaneous options
Misc:
  Update Time: 6        # The time in minutes to wait to perform an update
  Minimum Listeners: 15 # Feeds below this value will never be processed
  State Feeds ID: 6     # The state to process extra feed from in an update. It is not set by default
  Maximum Feeds To Display: 10

# This controls the global spike values (which are used to determine if a feed is jumping in listeners)
Spike Percentages:
  # This is the jump multiplier required for a feed to be considered "jumping".
  # Ex: If a feed's listeners are 30% higher than its current average listeners, it will be displayed
  Jump Required: 0.3
  # This controls how much (as a multiplier) the jump required for a feed will increase when its listeners are less than 50
  Low Listener Increase: 0.02
  # This is used along with the "High Listener Decrease Per Listeners" value to control how much the jump required will decrease when a feed is jumping to encourage further notifications
  High Listener Decrease: 0.02
  # This will decrease the jump required multiplier by the "High Listener Decrease" value for every x listeners set by this value
  High Listener Decrease Per Listeners: 100.0

# This controls what spike values should be used on a specific day of the week. Note that this is empty by default
Weekday Spike Percentages:
  # As you would expect, any day of the week can be used here, and the values of each day are identical to the "Spike Percentages" category above
  - Saturday:
    Jump Required: 0.3
    High Listener Decrease: 0.03
  - Sunday:
    Jump Required: 0.3
    High Listener Decrease: 0.03

# Feed-specific settings that apply to a feed name, feed ID, state, or county. Note that this is empty by default
Feed Settings:
  # This setting will only apply to a feed named "Test Feed"
  - Name: Test Feed
    # This category is identical the "Spike Percentages" one above, but it only applies to this feed group
    Spike Percentages:
      Jump Required: 0.3
      High Listener Decrease: 0.02
    # This category is identical the "Weekday Spike Percentages" one above, but it only applies to this feed group
    Weekday Spike Percentages:
      - Monday:
        Jump Required: 0.3
      - Friday:
        Jump Required: 0.3
  # This setting works the same way as the one above, but takes a feed ID instead
  - ID: 5698
    Spike Percentages:
      Jump Required: 0.3
  # And this one applies to an entire county
  - County: Sacramento
    Spike Percentages:
      Jump Required: 0.3
  # And this one applies to an entire state
  - State ID: 6 # California
    Spike Percentages:
      Jump Required: 0.3

# Changes what order feeds are displayed in
Feed Sorting:
  # Selects what metric feeds should be sorted by. Value can either be "Jump" or "Listeners"
  Sort By: Listeners
  # Changes what order feeds will be displayed in. Value can either be "Descending" or "Ascending"
  Sort Order: Descending

# Prevents certain feeds from displaying. Can specify a feed ID, state ID, county, or name. It is empty by default
Blacklist:
  # Note that these are all different filters
  - ID: 0
  - State ID: 0
  - County: Nowhereville
  - Name: Test Feed
  - County: Nonexistentville

# Only allows certain feeds to display. Allows the same identification types as the blacklist above. It is empty by default
Whitelist:
  - ID: 0
  - State ID: 0
  - County: Nowhereville
  - Name: Test Feed
  - County: Nonexistentville

# This category contains settings for the average of every feed that isn't biased towards large jumps in listeners. You should very rarely ever have to adjust any of these
Unskewed Average:
  # How close the feed's current listeners have to be to the unskewed average to remove it
  Reset To Average Percentage: 0.15
  # How much the unskewed average should slowly inch towards the current feed average to avoid lingering around forever in some cases
  Adjust To Average Percentage: 0.0075
  # How many times a feed needs to jump consecutively for the unskewed average to be set
  Spikes Required: 1
  # How much (as a multiplier) the current listeners of a feed need to be above the saved average to set the unskewed average immediately
  Jump Required To Set: 4.0
```

It is also worth noting that the coniguration file is reloaded on every update, so you do not need to restart the application after making changes to it.