module Feed

open System.Net
open System.Text.RegularExpressions

type Feed = {
    Name      : string
    Listeners : int
    Info      : string option
}

let createFromHTML str =
    let create (m:Match) =
        let value (i:int) = m.Groups.[i].Value
        {
            Listeners = 1 |> value |> int
            Name      = 2 |> value
            Info      = if (4 |> value).Length > 0
                        then Some <| value 4
                        else None
        }

    let regex = """<td class="c m">(\d+).*?<a href="/listen/feed/\d+">(.+?)</a>(<br /><br /> <div class="messageBox">(.+?)</div>)?"""
    Regex.Matches(str, regex, RegexOptions.Compiled)
    |> Seq.cast<Match>
    |> Seq.map create
    |> Seq.toArray

let createFromURL (url : string) =
    use c = new WebClient()
    c.DownloadString(url)
        .Trim()
        .Replace("\n", " ")
    |> createFromHTML

let createNotif feed index numFeeds =
    let infoStr =
        match feed.Info with
        | Some s -> sprintf "\nInfo: %s" s
        | None -> ""

    Notification.createUpdate
        index
        numFeeds
        (sprintf "Name: %s\nListeners: %d%s" feed.Name feed.Listeners infoStr)