module Feed

open System.Text.RegularExpressions

type Feed = {
    Name: string;
    Listeners: int;
    Info: string option;
}

let create (m:Match) =
    let groups = m.Groups
    let vFromIdx (i:int) = groups.[i].Value

    {
        Name      = 2 |> vFromIdx;
        Listeners = 1 |> vFromIdx |> int;
        Info      = if (4 |> vFromIdx).Length > 0
                    then Some (4 |> vFromIdx)
                    else None
    }

[<Literal>]
let private RegexString =
    """<td class="c m">(\d+).*?<a href="/listen/feed/\d+">(.+?)</a>(<br /><br /> <div class="messageBox">(.+?)</div>)?"""

let createAllFromString str =
    let matches =
        Regex.Matches(str, RegexString, RegexOptions.Compiled)
        |> Seq.cast<Match>
        |> Seq.toArray

    matches |> Array.map create