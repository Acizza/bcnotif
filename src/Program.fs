module Main

open System
open System.Collections.Generic
open System.Threading
open Feed
open Average

let showFeeds feeds =
    let create i f =
        let idx =
            // Reverse notif. index on Windows to accommodate for its display order
            #if WINDOWS
            feeds.Length-i
            #else
            i+1
            #endif

        Feed.createNotif f idx (Array.length feeds)

    feeds
    #if WINDOWS // Windows displays notifications in a reversed order
    |> Array.rev
    #endif
    |> Array.iteri create

let updateAverages hour (avgs : Dictionary<_, _>) =
    Array.iter (fun f ->
        let update = Average.update hour 5 f.Listeners
        let name   = f.Name

        if avgs.ContainsKey name
        then avgs.[name] <- avgs.[name] |> update
        else avgs.Add(name, Average.create 24 |> update)
    )

/// Filters feeds by a threshold and displays them as notifications
let processFeeds threshold (avgs : Dictionary<_, _>) =
    let feeds = Feed.createFromURL "http://www.broadcastify.com/listen/top"
    let hour  = DateTime.UtcNow.Hour

    let isPastAverage f avg =
        float f.Listeners >= float avg.Hourly.[hour] * threshold

    updateAverages hour avgs feeds

    feeds
    |> Array.choose (fun f ->
        match avgs.TryGetValue f.Name with
        | (true, avg) -> Some (f, avg)
        | (false, _)  -> None
    )
    |> Array.filter (fun (f, avg) -> isPastAverage f avg || Option.isSome f.Info)
    |> Array.map fst
    |> showFeeds
    
    avgs

let processLoop threshold (updateTime : TimeSpan) =
    let rec update avgs =
        let newAvgs = processFeeds threshold avgs

        newAvgs
        |> Seq.map (fun (KeyValue(n, avg)) -> (n, avg))
        |> Average.saveHourly

        Thread.Sleep (int updateTime.TotalMilliseconds)
        update newAvgs

    let avgs = new Dictionary<string, Average.T>()

    Average.loadHourly ()
    |> Seq.iter (fun (n, avg) ->
        if avgs.ContainsKey n |> not
        then avgs.Add(n, Average.createWithHourly avg)
    )

    update avgs

[<EntryPoint>]
let main args =
    let tryParse f input =
        match f input with
        | (true, x) -> Some x
        | (false, _) -> None

    let (|Minutes|_|) =
        tryParse Double.TryParse
        >> Option.map TimeSpan.FromMinutes

    let (|Threshold|_|) =
        tryParse Double.TryParse
        >> Option.map (fun x -> x / 100. + 1.)

    match args with
    | [|_; Minutes time|] when time.TotalMinutes < 5. ->
        Console.WriteLine "Update time must be >= 5 minutes"
    | [|Threshold threshold; Minutes updateTime|] ->
        processLoop threshold updateTime
    | _ ->
        Console.WriteLine "Usage: <percentage jump to display feed> <update time in minutes>"
        processLoop 30. (TimeSpan.FromMinutes 6.)
    0