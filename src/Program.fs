module Program

open System
open System.Net

[<Literal>]
let URL = "http://broadcastify.com/listen/top"

let sanitize (str:string) = str.Trim().Replace("\n", " ")

let downloadFeedList (url:string) =
    use c = new WebClient()
    c.DownloadString url |> sanitize

let fetchFeeds = downloadFeedList >> Feed.createAllFromString

let processFeeds minListeners =
    let feeds =
        fetchFeeds URL
        |> Array.filter (fun x -> x.Listeners >= minListeners)

    let displayRawFeed idx =
            Feed.prettify
            >> Notification.createFeedUpdate (idx+1) feeds.Length

    feeds
    |> Array.iteri displayRawFeed

let rec processLoopEvery minListeners (delay:TimeSpan) = async {
    try
        processFeeds minListeners
    with
    | ex -> Notification.createError ex.Message

    do! Async.Sleep (delay.TotalMilliseconds |> int)
    return! processLoopEvery minListeners delay
}

[<EntryPoint>]
let main args =
    let tryParse f input =
        match f input with
        | true,x -> Some x
        | false,_ -> None

    let (|Double|_|) = tryParse Double.TryParse
    let (|Int|_|)    = tryParse Int32.TryParse

    let start minListeners minsToUpdate =
        let t = TimeSpan.FromMinutes minsToUpdate
        Async.RunSynchronously (processLoopEvery minListeners t)

    match args |> Array.toList with
    | _::(Double minsToUpdate)::_ when minsToUpdate < 5. ->
        printfn "Update time must be >= 5 minutes."
    | (Int minListeners)::(Double minsToUpdate)::_ ->
        start minListeners minsToUpdate
    | _ ->
        start 325 10.

    0