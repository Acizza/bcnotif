module Feed

open System
open System.Net
open System.Text.RegularExpressions
open System.IO
open FSharp.Data

type Feed = {
    Name      : string
    Listeners : int
    Info      : string option
}

/// Creates an array of feeds from HTML
let createFromString str =
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
    |> createFromString

let createNotif feed index numFeeds =
    let infoStr =
        match feed.Info with
        | Some s -> sprintf "\nInfo: %s" s
        | None -> ""

    Notification.createUpdate
        index
        numFeeds
        (sprintf "Name: %s\nListeners: %d%s" feed.Name feed.Listeners infoStr)

module Averages =
    let filePath =
        AppDomain.CurrentDomain.BaseDirectory
        + "prevlisteners.csv"

    /// Saves hourly averages to a CSV file
    let saveHourly (avgs : (string * int array) seq) =
        let data =
            avgs
            //|> Seq.filter (fun (_, avg) -> avg.Length = 25)
            |> Seq.map (fun (name, avg) ->
                let listeners = Array.map string avg
                sprintf "\"%s\",%s"
                    <| name.Trim()
                    <| String.concat "," listeners
            )
            |> Array.ofSeq

        File.WriteAllLines(filePath, data)

    /// Creates the file specified if it doesn't exist, returns the length otherwise
    let private touchFile path =
        let info = FileInfo path

        match info.Exists with
        | true -> info.Length
        | false ->
            use f = File.Create(path)
            0L

    /// Loads the CSV file containing hourly averages
    let loadHourly () =
        let length = touchFile filePath

        if length <> 0L then
            CsvFile.Load(filePath, hasHeaders=false).Rows
            |> Seq.map (fun xs ->
                let columns =
                    xs.Columns.[1..]
                    |> Array.map int

                (xs.[0], columns)
            )
        else
            seq []