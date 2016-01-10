module Feed

open System
open System.Text.RegularExpressions
open System.IO
open FSharp.Data

type Feed = {
    name      : string
    listeners : int
    info      : string option
} with
    override x.ToString() =
        let maybe def f = function
            | Some x -> f x
            | None -> def

        sprintf "Name: %s\nListeners: %d%s"
            x.name
            x.listeners
            (maybe "" (sprintf "\nInfo: %s") x.info)

/// Creates an array of feeds from HTML
let createFromString str =
    let create (m:Match) =
        let value (i:int) = m.Groups.[i].Value
        {
            listeners = 1 |> value |> int
            name      = 2 |> value
            info      = if (4 |> value).Length > 0
                        then Some <| value 4
                        else None
        }

    let regex = """<td class="c m">(\d+).*?<a href="/listen/feed/\d+">(.+?)</a>(<br /><br /> <div class="messageBox">(.+?)</div>)?"""
    Regex.Matches(str, regex, RegexOptions.Compiled)
    |> Seq.cast<Match>
    |> Seq.toArray
    |> Array.map create

let getAverageListeners feeds =
    feeds
    |> Array.concat
    |> Array.groupBy (fun x -> x.name)
    |> Array.map (fun (name, arr) ->
        (name, Array.averageBy (fun x -> float x.listeners) arr)
    )
    |> Map.ofArray

module PreviousData =
    let filePath =
        AppDomain.CurrentDomain.BaseDirectory
        + "prevlisteners.csv"

    /// Transforms an array of feeds to a CSV-friendly format and
    /// writes them to the specified path
    let save path feeds =
        let data =
            feeds
            |> Array.concat
            |> Array.groupBy (fun x -> x.name)
            |> Array.map (fun (name, values) ->
                (name, Array.map (fun x -> string x.listeners) values)
            )
            |> Array.filter (fun (_, listeners) -> listeners.Length >= 5) // TODO: Hard-coded number of averages!
            |> Array.map (fun (name, listeners) ->
                (sprintf "\"%s\"," <| name.Trim()) + (String.concat "," listeners)
            )

        File.WriteAllLines(path, data)

    /// Loads data from the specified CSV file
    let load (path:string) =
        // Avoids an exception with an empty file
        if (new FileInfo(path)).Length <> 0L then
            let csv = CsvFile.Load(path, hasHeaders=false).Cache()
            csv.Rows
            |> Seq.toArray
            |> Array.map (fun xs -> (xs.[0], xs.Columns.[1..]))
        else
            [||]

    /// Loads data from the specified CSV file and transforms them into an array of feeds
    let loadToFeeds path =
        load path
        |> Array.map (fun (name, values) ->
            Array.map (fun listeners ->
                {
                    name      = name
                    listeners = int listeners
                    info      = None
                }) values
        )

    /// Loads feeds from the specified CSV file if it exists; otherwise,
    /// creates the file with the specified feeds
    let getOrCreate path feeds =
        match File.Exists path with
        | true  -> loadToFeeds path
        | false ->
            save path feeds
            [||]
