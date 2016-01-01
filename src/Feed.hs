{-# LANGUAGE QuasiQuotes, FlexibleContexts #-}

module Feed
( Feed(..)
, createFromString
) where

import Data.Maybe (mapMaybe)
import Data.String.Utils (replace)
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

create :: [String] -> Maybe Feed
create [listeners', name'] = Just $ Feed name' (read listeners') Nothing
create [listeners', name', _, info'] = Just $ Feed name' (read listeners') (Just info')
create _ = Nothing

getMatches :: String -> [(String, [String])]
getMatches = scan [re|<td class="c m">(\d+).*?<a href="/listen/feed/\d+">(.+?)</a>(<br /><br /> <div class="messageBox">(.+?)</div>)?|]

createFromString :: String -> [Feed]
createFromString = mapMaybe (create . snd) . getMatches . replace "\n" " "
