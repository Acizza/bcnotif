module Feed

open System
open System.Text.RegularExpressions
open System.IO
open FSharp.Data

type Feed = {
    Name      : string
    Listeners : int
    Info      : string option
} with
    override x.ToString() =
        let maybe def f = function
            | Some x -> f x
            | None -> def

        sprintf "Name: %s\nListeners: %d%s"
            x.Name
            x.Listeners
            (maybe "" (sprintf "\nInfo: %s") x.Info)

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
    |> Seq.toArray
    |> Array.map create

let getAverageListeners feeds =
    feeds
    |> Array.concat
    |> Array.groupBy (fun x -> x.Name)
    |> Array.map (fun (name, arr) ->
        (name, Array.averageBy (fun x -> float x.Listeners) arr)
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
            |> Array.groupBy (fun x -> x.Name)
            |> Array.map (fun (name, values) ->
                (name, Array.map (fun x -> string x.Listeners) values)
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
                    Name      = name
                    Listeners = int listeners
                    Info      = None
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