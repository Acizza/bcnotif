module Program

open System
open System.Net

[<Literal>]
let URL = "http://broadcastify.com/listen/top"

let sanitize (str:string) = str.Trim().Replace("\n", " ")
let downloadFeedList (url:string) = (new WebClient()).DownloadString url |> sanitize
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

[<EntryPoint>]
let main args =
    let tryParse f input =
        match f input with
        | true,x -> Some x
        | false,_ -> None

    let (|Double|_|) = tryParse Double.TryParse
    let (|Int|_|)    = tryParse Int32.TryParse

    match args |> Array.toList with
    | (Int minListeners)::(Double updateTime)::_ ->
        processFeeds minListeners
    | _ ->
        printfn "Usage: <minimum listeners> <update time in minutes>"

    0