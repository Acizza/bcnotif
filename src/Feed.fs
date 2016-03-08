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
    type T = Map<string, int list>

    let filePath =
        AppDomain.CurrentDomain.BaseDirectory
        + "prevlisteners.csv"

    let getAverageListeners feed avgs =
        Map.tryFind feed.Name avgs
        |> Option.map (List.averageBy float)

    let isPastAverage feed avgs =
        match getAverageListeners feed avgs with
        | Some avg -> float feed.Listeners >= avg * 1.3
        | None -> false

    /// Transforms a map containing average listeners and saves it to the CSV file
    let save (avgs : T) =
        let data =
            avgs
            |> Map.toArray
            |> Array.filter (fun (_, l) -> l.Length >= 5)
            |> Array.map (fun (name, listeners) ->
                let listeners = List.map string listeners

                sprintf "\"%s\",%s"
                    <| name.Trim()
                    <| String.concat "," listeners
            )

        File.WriteAllLines(filePath, data)

    /// Creates the file specified if it doesn't exist, returns the length otherwise
    let private touchFile path =
        let info = FileInfo path

        match info.Exists with
        | true -> info.Length
        | false ->
            use f = File.Create(path)
            0L

    /// Loads the CSV file containing previous listener data
    let load () =
        let length = touchFile filePath

        if length <> 0L then
            CsvFile.Load(filePath, hasHeaders=false).Rows
            |> Seq.map (fun xs ->
                let columns =
                    xs.Columns.[1..]
                    |> Array.map int
                    |> Array.toList

                (xs.[0], columns)
            )
            |> Map.ofSeq
        else
            Map.empty