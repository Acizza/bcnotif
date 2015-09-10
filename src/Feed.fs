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
        Name      = 1 |> vFromIdx;
        Listeners = 2 |> vFromIdx |> int;
        Info      = if groups.Count >= 4
                    then Some (4 |> vFromIdx)
                    else None
    }
