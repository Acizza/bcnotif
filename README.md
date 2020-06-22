# bcnotif

[![total lines](https://tokei.rs/b1/github/acizza/bcnotif)](https://github.com/acizza/bcnotif)

This is a Linux program to find audio feeds on Broadcastify that suddenly jump in listeners (or have an alert) and display a desktop notification for them, as a sudden jump in listeners usually means that some kind of event or emergency is happening. Since some people tune into feeds as soon as they see / hear an incident occuring, you can usually get notified of large incidents (such as mass shootings) faster than you would following the news.

# Building

This project requires the following dependencies:

* A recent stable version of Rust
* SQLite
* dbus
* pkg-config

Note that pkg-config is most likely already installed. If your distribution does not provide a recent version of Rust, you can obtain the latest version [here](https://rustup.rs/).

Once the dependencies are installed, you can build the project simply by running `cargo build --release` in the project's directory. Once compilation is complete, you will find the `bcnotif` binary in the `target/release/` folder. None of the other files in that directory need to be kept.

# Usage

This program runs in the background, so it can be launched and forgotten about. Note that if you plan on configuring things, you will either have to kill & relaunch the program after saving changes or launch the program initially with the `-r` flag.

# Configuration

To configure the program, first create and open the file at `~/.config/bcnotif/config.toml`.

In addition to the top 50 feeds on Broadcastify, you can set a specific location that you also want to be processed during an update. This option can be specified like so in your configuration file:

```toml
[misc]
process_location = "us-california"
```

The value of the `process_location` field follows the following format:

`<country>-<state/province/territory in kebab case>`

For example, New York would look like this:

`us-new-york`

And New South Wales in Austrailia would look like this:

`au-new-south-wales`

Currently, You can only specify locations in the following countries:

* Austrailia
* Brazil
* Canada
* Chile
* Luxembourg
* Malaysia
* Netherlands
* United States

## Feed Selectors

You can modify how the program processes feeds matching certain "selectors", as well as how they are processed on specific weekdays. These selectors currently include a feed's ID, county, location name, as well as a global selector to match any feed. This system makes it very easy to make feeds located in your state or county more (or less) sensitive to listener jumps.

The following example will require all feeds located in "Sacramento County" to jump in listeners by 50% in order to show an alert:

```toml
[feed."county(Sacramento County)"]
jump_required = 50
```

You can also use this system to only apply your feed settings on specific days of the week. The following example does the same thing as the one above, but it will only be applied on Wednesday:

```toml
[weekday.wednesday."county(Sacramento County)"]
jump_required = 50
```

Same as both examples above, but applied to the entire state of New York:

```toml
[feed."location(us-new-york)"]
jump_required = 50

[weekday.wednesday."location(us-new-york)"]
jump_required = 50
```

## Full Configuration File Example

The following shows a complete configuration file, will all options filled in various configurations:

```toml
# The feed with ID 123 will need to jump by 47.23% on Saturday in order to show an alert for it.
# When specifying weekdays, you can use either the short or long name.
[weekday.sat."id(123)"]
jump_required = 47.23

# All feeds on Sunday will need to jump by 70% in order to show alerts for them.
[weekday.sunday.global]
jump_required = 70

# This is the default jump percentage used by all feeds.
[feed.global]
jump_required = 40

# All feeds in California will only have to jump by 35% in order to show an alert for them.
[feed."location(us-california)"]
jump_required = 35

[misc]
# How often to run feed updates in minutes. This is the default.
update_time_mins = 6
# The minimum number of listeners a feed must have to process it. This is the default.
minimum_listeners = 15
# The location to process in addition to the top 50 feeds. This is not set by default.
process_location = "us-california"
# The maximum number of feeds to display an alert for at once. This is the default.
show_max = 10
# The maximum number of times to show a feed that's alerting consecutively. This is not set by default.
show_max_times = 5
# Specifies whether or not feeds that have an alert attached to them should be shown regardless of them spiking in listeners. This is the default. Possible values are "true" and "false".
show_alert_feeds = true

# This section controls the order notifications are shown for feeds.
[sorting]
# The value to sort feeds by. This is the default. Possible values are "jump" and "listeners".
# The "jump" value means that feeds are sorted by how large their listener jump is.
value = "jump"
# The order to sort the feeds in, based off the specified value field above. This is the default. Possible values are "descending" and "ascending".
order = "descending"

# This section allows you to blacklist and whitelist feeds, using the same selectors that are used in the feed and weekday sections.
[filters]
# This will prevent the feed with ID 1, feeds in the county "example county", and all feeds in Alabama from ever showing. This is not set by default.
blacklist = [ "id(1)", "county(example county)", "location(us-alabama)" ]
# This only allows feeds in Alaska and the feed with ID 123 to ever show. This is not set by default.
whitelist = [ "location(us-alaska)", "id(123)" ]
```