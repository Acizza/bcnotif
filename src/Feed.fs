module Feed

open System.Text.RegularExpressions

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
