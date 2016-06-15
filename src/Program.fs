module Main

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

let showFeeds feeds =
    let create i f =
        let idx =
            // Reverse notif. index on Windows to accommodate for its display order
            #if WINDOWS
            Array.length feeds - i
            #else
            i+1
            #endif

        Feed.createNotif f idx (Array.length feeds)

    feeds
    #if WINDOWS // Windows displays notifications in a reversed order
    |> Array.rev
    #endif
    |> Array.iteri create

let localPath path =
    sprintf "%s%s"
        AppDomain.CurrentDomain.BaseDirectory
        path

let averagesPath   = localPath "averages.csv"
let thresholdsPath = localPath "thresholds.csv"

let update threshold avgs =
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

    let thresholds = Threshold.loadFromFile thresholdsPath

    let isPastAverage f avg =
        let threshold =
            thresholds
            |> Map.tryFind f.Name
            |> Option.defaultArg threshold
        float f.Listeners >= float avg.Hourly.[hour] * threshold

    feeds
    |> Array.filter (fun (f, avg) -> isPastAverage f avg || Option.isSome f.Info)
    |> Array.map fst
    |> showFeeds

    let newAvgs =
        feeds
        |> Array.fold (fun m (f, avg) -> Map.add f.Name avg m) avgs

    Average.saveToFile averagesPath newAvgs
    newAvgs

let start threshold (updateTime : TimeSpan) =
    let rec loop avgs =
        let newAvgs = update threshold avgs
        Thread.Sleep (int updateTime.TotalMilliseconds)
        loop newAvgs

    Average.loadFromFile averagesPath
    |> loop

[<EntryPoint>]
let main args =
    let (|Minutes|_|) =
        Convert.tryParse Double.TryParse
        >> Option.map TimeSpan.FromMinutes

    let (|Threshold|_|) =
        Convert.tryParse Double.TryParse
        >> Option.map (fun x -> x / 100. + 1.)

    match args with
    | [|_; Minutes time|] when time.TotalMinutes < 5. ->
        printfn "Update time must be >= 5 minutes"
    | [|Threshold threshold; Minutes updateTime|] ->
        start threshold updateTime
    | _ ->
        printfn "Usage: <percentage jump to display feed> <update time in minutes>"
        start 30. (TimeSpan.FromMinutes 6.)
    0