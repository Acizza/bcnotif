{-# LANGUAGE ViewPatterns #-}

module Main where

import Control.Concurrent (threadDelay)
import System.Environment (getArgs)
import Text.Read (readMaybe)
import Network.HTTP (simpleHTTP, getRequest, getResponseBody)
import Text.Printf (printf)
import qualified Notification as N
import Feed

downloadURL :: String -> IO String
downloadURL url = simpleHTTP (getRequest url) >>= getResponseBody

getFeeds :: Int -> String -> [Feed]
getFeeds minListeners =
    filter (\x -> listeners x > minListeners) . Feed.createFromString

startUpdateLoop :: Int -> Int -> IO ()
startUpdateLoop minListeners minsToUpdate = do
    str <- downloadURL "http://www.broadcastify.com/listen/top"

    let feeds = zip [1..] . getFeeds minListeners $ str
    let getTitle i = printf "Broadcastify Listener Update (%d of %d)" (i :: Int) (length feeds :: Int)
    mapM_ (\(i,x) -> N.createFeedUpdate (getTitle i) (show x)) feeds

    threadDelay $ 1000000 * 60 * minsToUpdate
    startUpdateLoop minListeners minsToUpdate

main :: IO ()
main = do
    args <- getArgs
    case args of
        [_, readMaybe -> Just minsToUpdate] | minsToUpdate < 5.0 ->
            putStrLn "Update time must be >= 5 minutes"
        [readMaybe -> Just minListeners, readMaybe -> Just minsToUpdate] ->
            startUpdateLoop minListeners minsToUpdate
        otherwise -> do
            putStrLn "Usage: <minimum listeners> <update time in minutes>"
            startUpdateLoop 325 10
