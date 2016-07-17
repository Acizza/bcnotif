module Main

open Argu
open Average
open Feed
open System
open System.Threading
open Util

module Threshold =
    open FSharp.Data
    open System.IO

    type Thresholds = CsvProvider<Schema = "Feed (string), Threshold (float)", HasHeaders = false>

    let loadFromFile path =
        match File.Exists path && (FileInfo path).Length > 0L with
        | true ->
            Thresholds.Load(uri = path).Rows
            |> Seq.map (fun row -> (row.Feed, row.Threshold))
            |> Map.ofSeq
        | false -> Map.empty

module Path =
    let getLocal path =
        sprintf "%s%s"
            AppDomain.CurrentDomain.BaseDirectory
            path

    let averages   = getLocal "averages.csv"
    let thresholds = getLocal "thresholds.csv"

module Args =
    type SortOrder =
        | Ascending
        | Descending

    type Arguments =
        | [<AltCommandLine("-p")>] Threshold  of percentage:float
        | [<AltCommandLine("-t")>] UpdateTime of minutes:float
        | [<AltCommandLine("-s")>] Sort       of SortOrder
        with
            interface IArgParserTemplate with
                member s.Usage =
                    match s with
                    | Threshold  _ -> "specify the percentage jump required to show a feed"
                    | UpdateTime _ -> "specify the minutes between updates (must be >= 5)"
                    | Sort       _ -> "specify the order feeds will be displayed in"

open Args

let showFeeds sortOrder feeds =
    let sort =
        match sortOrder with
        | Ascending  -> Array.rev
        | Descending -> id

    feeds
    |> Array.mapi (fun i f -> (i, f))
    |> sort
    |> Array.iter (fun (i, f) -> Feed.createNotif f (i+1) (Array.length feeds))

let update threshold sortOrder avgs =
    let hour  = DateTime.UtcNow.Hour
    let feeds =
        Feed.createFromURL "http://www.broadcastify.com/listen/top"
        |> Array.map (fun f ->
            let avg =
                Map.tryFind f.Name avgs
                |> Option.defaultArg (Average.create 24)
                |> Average.update hour 5 f.Listeners
            (f, avg)
        )
        
    let thresholds = Threshold.loadFromFile Path.thresholds

    let isPastAverage f avg =
        let threshold =
            thresholds
            |> Map.tryFind f.Name
            |> Option.defaultArg threshold
        float f.Listeners >= float avg.Hourly.[hour] * threshold

    feeds
    |> Array.filter (fun (f, avg) -> isPastAverage f avg || Option.isSome f.Info)
    |> Array.map fst
    |> showFeeds sortOrder

    let newAvgs =
        feeds
        |> Array.fold (fun m (f, avg) -> Map.add f.Name avg m) avgs

    Average.saveToFile Path.averages newAvgs
    newAvgs

let start threshold (updateTime : TimeSpan) sortOrder =
    let rec loop avgs =
        let newAvgs = update threshold sortOrder avgs
        Thread.Sleep (int updateTime.TotalMilliseconds)
        loop newAvgs

    Average.loadFromFile Path.averages
    |> loop

[<EntryPoint>]
let main args =
    let parser = ArgumentParser.Create<Arguments>()
    try
        let results = parser.Parse args

        let threshold  = results.GetResult(<@ Threshold @>,  defaultValue = 30.) / 100. + 1.
        let updateTime = results.GetResult(<@ UpdateTime @>, defaultValue = 6.)
        let sortOrder  = results.GetResult(<@ Sort @>,       defaultValue = Descending)

        let (|Minutes|) = TimeSpan.FromMinutes

        match updateTime with
        | t when t < 5. -> raise (ArgumentOutOfRangeException())
        | Minutes time  -> start threshold time sortOrder
    with
    | :? ArguParseException | :? ArgumentOutOfRangeException ->
        parser.PrintUsage() |> eprintfn "%s"
    | ex -> eprintfn "%A" ex

    0