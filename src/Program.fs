module Main

open System
open System.Threading
open Feed

type Averages = {
    Moving : int list
    Hourly : int array
}

let showFeeds feeds =
    let create i f =
        let idx =
            // Reverse notification index on Windows to accommodate for its display order
            #if WINDOWS
            feeds'.Length-i
            #else
            i+1
            #endif

        Feed.createNotif f idx (Array.length feeds)

    feeds |> Array.iteri create

let updateFeedAvg curHour avg feed =
    let hourlyAvg = avg.Hourly.[curHour]
    let newMoving =
        match avg.Moving with
        | xs when xs.Length < 5 -> feed.Listeners :: xs
        | xs -> feed.Listeners :: xs.[..xs.Length - 2]
        | _  -> if hourlyAvg = 0 then [feed.Listeners] else [hourlyAvg]

    avg.Hourly.[curHour] <- newMoving |> List.averageBy float |> int
    {avg with Moving = newMoving}

/// Filters feeds by a threshold and displays them as notifications
let processFeeds threshold avgs =
    let feeds = Feed.createFromURL "http://www.broadcastify.com/listen/top"
    let hour  = DateTime.UtcNow.Hour

    let newAvgs =
        Array.map (fun f ->
            let avg =
                match Array.tryFind (fun (n, _) -> n = f.Name) avgs with
                | Some (_, x) -> x
                | None -> {Moving = []; Hourly = Array.zeroCreate 25}
            (f, updateFeedAvg hour avg f)
        ) feeds
        
    newAvgs
    |> Array.filter (fun (f, avg) ->
        float f.Listeners >= float avg.Hourly.[hour] * threshold ||
        Option.isSome f.Info
    )
    #if WINDOWS // Windows displays notifications as FILO; Linux displays them as FIFO
    |> Array.rev
    #endif
    |> Array.map fst
    |> showFeeds

    newAvgs
    |> Array.map (fun (f, avg) -> (f.Name, avg))

let processLoop threshold (updateTime : TimeSpan) =
    let rec update avgs =
        let newAvgs = processFeeds threshold avgs

        newAvgs
        |> Array.map (fun (n, avg) -> (n, avg.Hourly))
        |> Averages.saveHourly

        Thread.Sleep (int updateTime.TotalMilliseconds)
        update newAvgs

    let initialAverages =
        Averages.loadHourly ()
        |> Seq.map (fun (f, avgs) ->
            (f, { Moving = []; Hourly = avgs })
        )
        |> Seq.toArray

    update initialAverages

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