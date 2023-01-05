{-# LANGUAGE BlockArguments #-}
{-# LANGUAGE DisambiguateRecordFields #-}
{-# LANGUAGE ImportQualifiedPost #-}
{-# LANGUAGE LambdaCase #-}
{-# LANGUAGE OverloadedStrings #-}
{-# LANGUAGE PackageImports #-}
{-# LANGUAGE NoImplicitPrelude #-}
{-# OPTIONS_GHC -Weverything -fno-warn-missing-import-lists -fno-warn-incomplete-uni-patterns -fno-warn-missing-safe-haskell-mode -fno-warn-all-missed-specialisations -fno-warn-unsafe #-}

{-
ghcid --command='stack repl ditto-lsp-test'
-}

module Main (main) where

import "lsp-test" Language.LSP.Test qualified as Lsp
import "lsp-types" Language.LSP.Types as LspTypes
import "base" System.Environment (getArgs)
import "base" System.Exit (die)
import "process" System.Process qualified as Process
import "base" Prelude

main :: IO ()
main =
  getArgs >>= \case
    [] -> die "missing arguments"
    [lspExe] -> do
      putStrLn lspExe
      test lspExe
    _ -> die "too many arguments"

test :: String -> IO ()
test lspExe = do
  testSemanticTokens lspExe
  testFormatting lspExe
  testDiagnostics0 lspExe
  testDiagnostics1 lspExe

testSemanticTokens :: String -> IO ()
testSemanticTokens lspExe =
  runSession lspExe "crates/ditto-lsp/fixtures/semantic-tokens" do
    example <- Lsp.openDoc "Example.ditto" "ditto"
    Just LspTypes.SemanticTokens {} <- Lsp.getSemanticTokens example
    pure ()

testFormatting :: String -> IO ()
testFormatting lspExe =
  runSession lspExe "crates/ditto-lsp/fixtures/formatting" do
    example <- Lsp.openDoc "Example.ditto" "ditto"
    Lsp.formatDoc
      example
      LspTypes.FormattingOptions
        { -- NOTE: these options are currently ignored
          LspTypes._tabSize = 4,
          LspTypes._insertSpaces = False,
          LspTypes._trimTrailingWhitespace = Just True,
          LspTypes._insertFinalNewline = Just True,
          LspTypes._trimFinalNewlines = Just True
        }

    pure ()

testDiagnostics0 :: String -> IO ()
testDiagnostics0 lspExe =
  runSession lspExe "crates/ditto-lsp/fixtures/diagnostics-0" do
    _ <- Lsp.openDoc "ditto-src/A.ditto" "ditto"
    [ LspTypes.Diagnostic
        { _source =
            Just "ditto",
          _severity = Just LspTypes.DsError,
          _message = "types don't unify \nexpected String\ngot Int"
        }
      ] <-
      Lsp.waitForDiagnostics
    pure ()

testDiagnostics1 :: String -> IO ()
testDiagnostics1 lspExe =
  runSession lspExe "crates/ditto-lsp/fixtures/diagnostics-1" do
    _ <- Lsp.openDoc "ditto-src/A.ditto" "ditto"
    [ LspTypes.Diagnostic
        { _source =
            Just "ditto",
          _severity = Just LspTypes.DsError,
          _message = "types don't unify \nexpected String\ngot Int"
        }
      ] <-
      Lsp.waitForDiagnostics
    pure ()

runSession :: String -> FilePath -> Lsp.Session a -> IO a
runSession lspExe rootDir session = do
  Process.withCreateProcess proc \(Just stdin) (Just stdout) _ _ -> do
    Lsp.runSessionWithHandles stdin stdout sessionConfig Lsp.fullCaps rootDir session
  where
    proc =
      (Process.proc lspExe [])
        { Process.std_in = Process.CreatePipe,
          Process.std_out = Process.CreatePipe,
          Process.cwd = Just rootDir
        }
    sessionConfig =
      Lsp.defaultConfig
        { Lsp.messageTimeout = 5,
          Lsp.logStdErr = True,
          Lsp.logMessages = False
        }
