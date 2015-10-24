{-# LANGUAGE QuasiQuotes, FlexibleContexts #-}

module Feed
    ( Feed(..)
    , create
    , createFromString
    , getMatches
    ) where

import Data.Maybe (maybe)
import Text.Printf (printf)
import Text.Regex.PCRE.Heavy

data Feed = Feed
    { name      :: String
    , listeners :: Int
    , info      :: Maybe String
    }

instance Show Feed where
    show f =
        printf "Name: %s\nListeners: %d%s"
            (name f)
            (listeners f)
            (maybe "" (printf "\nInfo: %s") (info f))

create :: [String] -> Feed
create [listeners, name] = Feed name (read listeners) Nothing
create [listeners, name, _, info] = Feed name (read listeners) (Just info)
create other = error $ "create usage: [listeners, name] or [listeners, name, info] " ++ show other

getMatches :: String -> [(String, [String])]
getMatches = scan [re|<td class="c m">(\d+).*?<a href="/listen/feed/\d+">(.+?)</a>(<br /><br /> <div class="messageBox">(.+?)</div>)?|]

replaceLines :: String -> String
replaceLines str =
    let
        rep '\n' = ' '
        rep c = c
    in map rep str

createFromString :: String -> [Feed]
createFromString = map (create . snd) . getMatches . replaceLines
