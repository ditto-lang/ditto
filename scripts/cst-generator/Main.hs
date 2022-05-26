{-# LANGUAGE BlockArguments #-}
{-# LANGUAGE ImportQualifiedPost #-}
{-# LANGUAGE OverloadedLists #-}
{-# LANGUAGE PackageImports #-}
{-# LANGUAGE Safe #-}
{-# LANGUAGE ScopedTypeVariables #-}
{-# LANGUAGE TypeApplications #-}
{-# LANGUAGE NoImplicitPrelude #-}
{-# OPTIONS_GHC -Weverything -fno-warn-missing-import-lists -fno-warn-all-missed-specialisations #-}

module Main (main) where

import "base" Control.Monad (foldM, join, replicateM)
import "transformers" Control.Monad.Trans.State qualified as State
import "base" Data.List (intercalate)
import "base" Data.List.NonEmpty (NonEmpty (..))
import "base" Data.List.NonEmpty qualified as NonEmpty
import "base" Data.Maybe (catMaybes)
import "base" System.IO (hPutStrLn, stderr)
import "random" System.Random (RandomGen, initStdGen, uniformR)
import "base" Text.Printf (printf)
import "base" Prelude

lower :: RandomGen g => Random g Char
lower = choose ['a' .. 'z']

upper :: RandomGen g => Random g Char
upper = choose ['A' .. 'Z']

wordChar :: RandomGen g => Random g Char
wordChar = choose $ '_' :| ['A' .. 'Z'] <> ['a' .. 'z']

properName :: RandomGen g => Random g String
properName = (:) <$> upper <*> list (1, 10) wordChar

name :: RandomGen g => Random g String
name = notKeyword ((:) <$> lower <*> list (1, 10) wordChar)

unusedName :: RandomGen g => Random g String
unusedName = fmap ("_" <>) name

qualified :: RandomGen g => Random g String -> Random g String
qualified rand =
  chooseRandom
    [ rand,
      sep "." <$> sequence [properName, rand]
    ]

qualifiedName :: RandomGen g => Random g String
qualifiedName = qualified name

qualifiedProperName :: RandomGen g => Random g String
qualifiedProperName = qualified properName

everything :: String
everything = "(..)"

exportList :: RandomGen g => Random g String
exportList =
  chooseRandom
    [ pure everything,
      exposeList
    ]

exposeList :: forall g. RandomGen g => Random g String
exposeList = do
  items <- list (5, 15) exposeItem
  pure $ parens (commaSep items)
  where
    exposeItem = chooseRandom [exposeValue, exposeType]

    exposeValue :: Random g String
    exposeValue = name

    exposeType :: Random g String
    exposeType = do
      typeName <- properName
      abstract <- bool
      pure if abstract then typeName else (typeName <> everything)

importLine :: forall g. RandomGen g => Random g String
importLine =
  unwords . catMaybes
    <$> sequence
      [ pure (Just "import"),
        optional packageName,
        Just <$> moduleName,
        optional alias,
        optional exposeList,
        pure (Just ";")
      ]
  where
    packageName :: Random g String
    packageName = notKeyword do
      let char :: Random g Char
          char = choose $ '-' :| ['a' .. 'z']
      s <- (:) <$> lower <*> list (1, 10) char
      pure (parens s)

    alias :: Random g String
    alias = fmap ("as " <>) properName

moduleName :: RandomGen g => Random g String
moduleName = sep "." <$> list (1, 10) properName

moduleDeclaration :: forall g. RandomGen g => Random g String
moduleDeclaration =
  chooseRandom
    [ foreignDeclaration,
      typeDeclaration,
      valueDeclaration
    ]
  where
    foreignDeclaration :: Random g String
    foreignDeclaration = do
      n <- name
      t <- dittoType 4
      pure $ unwords ["foreign", n, ":", t, ";"]

    typeDeclaration :: Random g String
    typeDeclaration = do
      typeName <- properName
      params <- maybe "" (parens . commaSep) <$> optional (list (1, 6) name)
      constructors <- list (0, 10) do
        constructorName <- properName
        args <- maybe "" (parens . commaSep) <$> optional (list (1, 6) (dittoType 2))
        pure (constructorName <> args)
      case constructors of
        [] -> pure $ unwords ["type", typeName, params, ";"]
        [constructor] -> do
          includePipe <- bool
          pure $ unwords ["type", typeName, params, "=", if includePipe then "|" else "", constructor, ";"]
        _ ->
          pure $ unwords ["type", typeName, params, "=", unlines (map ("| " <>) constructors), ";"]

    valueDeclaration :: Random g String
    valueDeclaration = do
      valueName <- name
      typeAnn <- maybe "" (": " <>) <$> optional (dittoType 4)
      value <- expr 4
      pure $ unwords [valueName, typeAnn, "=", value, ";"]

moduleHeader :: RandomGen g => Random g String
moduleHeader = do
  mn <- moduleName
  exports <- exportList
  pure $ unwords ["module", mn, "exports", exports, ";"]

dittoType :: forall g. RandomGen g => Int -> Random g String
dittoType depth
  | depth <= 0 = type0
  | otherwise =
    chooseRandom
      [ typeParens,
        typeFunction,
        typeCall,
        typeClosedRecord,
        typeOpenRecord,
        type0
      ]
  where
    typeParens = parens <$> child

    typeCall = do
      fn <- chooseRandom [typeVariable, typeConstructor]
      params <- list (1, 5) child
      pure $ fn <> parens (commaSep params)

    typeFunction = do
      params <- list (0, 4) child
      returned <- child
      pure $ parens (commaSep params) <> " -> " <> returned

    typeClosedRecord = do
      fields <- list (0, 5) typeRecordField
      pure $ braces (commaSep fields)
    typeOpenRecord = do
      var <- typeVariable
      fields <- list (1, 5) typeRecordField
      pure $ braces (var <> "|" <> commaSep fields)
    typeRecordField = (\label t -> label <> ": " <> t) <$> name <*> child

    child :: Random g String
    child = dittoType (depth - 1)

    type0 =
      chooseRandom
        [ typeVariable,
          typeConstructor
        ]

    typeVariable :: Random g String
    typeVariable = name

    typeConstructor :: Random g String
    typeConstructor = qualifiedProperName

expr :: forall g. RandomGen g => Int -> Random g String
expr depth
  | depth <= 0 = expr0
  | otherwise =
    chooseRandom
      [ parens <$> child,
        exprIf,
        exprCall,
        exprFn,
        exprMatch,
        exprEffect,
        exprAccess,
        exprArray,
        exprRecord,
        expr0
      ]
  where
    exprIf = unwords <$> sequence [pure "if", child, pure "then", child, pure "else", child]

    exprCall = do
      f <- chooseRandom [exprVariable, exprConstructor, exprAccess, parens <$> exprFn]
      args <- parens . commaSep <$> list (0, 20) child
      pure (f <> args)

    exprFn = do
      params <-
        parens . commaSep <$> list (0, 10) do
          binder <- name
          typeAnn <- maybe "" (": " <>) <$> optional (dittoType 4)
          pure (binder <> typeAnn)
      typeAnn <- maybe "" ((": " <>) . parens) <$> optional (dittoType 7)
      body <- child
      pure ("fn" <> params <> typeAnn <> " -> " <> body)

    exprMatch = do
      matched <- expr0
      arms <- list (1, 10) do
        pat <- pattern 4
        e <- child
        pure ("| " <> pat <> " -> " <> e)

      pure . unwords $ ["match", matched, "with"] <> arms <> ["end"]

    exprEffect = do
      n <- int (1, 20)
      fmap (("do " <>) . braces) (stmts n)
      where
        stmts :: Int -> Random g String
        stmts n
          | n <= 0 = chooseRandom [stmtReturn, stmtExpr]
          | otherwise = stmtBind n

        stmtBind n = do
          binder <- name
          e <- expr0
          rest <- stmts (n - 1) -- not tail recursive but meh for now
          pure $ unwords [binder, "<-", e, ";", rest]

        stmtReturn = fmap ("return " <>) expr0
        stmtExpr = expr0

    exprAccess :: Random g String
    exprAccess = intercalate "." <$> list (2, 20) name

    exprRecord :: Random g String
    exprRecord =
      braces . commaSep <$> list (0, 10) do
        label <- name
        value <- child
        pure (label <> " = " <> value)

    exprArray :: Random g String
    exprArray = brackets . commaSep <$> list (0, 10) child

    child :: Random g String
    child = expr (depth - 1)

    expr0 =
      chooseRandom
        [ exprVariable,
          exprConstructor,
          exprString,
          exprInt,
          exprFloat,
          exprPipe,
          pure "unit",
          pure "true",
          pure "false"
        ]

    exprPipe = sep "|>" <$> list (1, 10) expr0

    exprVariable :: Random g String
    exprVariable = qualifiedName

    exprConstructor :: Random g String
    exprConstructor = qualifiedProperName

    exprInt :: Random g String
    exprInt = show <$> int (0, 100000)

    exprFloat :: Random g String
    exprFloat = do
      i <- int (1, 100000)
      j <- int (1, 100000)
      pure $ printf "%.4f" (fromIntegral @_ @Double i / fromIntegral j)

    exprString :: Random g String
    exprString =
      dquotes <$> list (10, 50) (choose $ ['0' .. '9'] <> ['A' .. 'Z'] <> ['a' .. 'z'])

pattern :: forall g. RandomGen g => Int -> Random g String
pattern depth
  | depth <= 0 = pattern0
  | otherwise = chooseRandom [patternConstructor, pattern0]
  where
    patternConstructor :: Random g String
    patternConstructor = do
      ctorName <- qualifiedProperName
      args <- parens . commaSep <$> list (1, 10) (pattern (depth - 1))
      pure (ctorName <> args)

    pattern0 :: Random g String
    pattern0 =
      chooseRandom
        [name, unusedName, qualifiedProperName]

_dittoModule :: RandomGen g => Random g String
_dittoModule = do
  header <- moduleHeader
  imports <- replicateM 20 importLine
  decls <- replicateM 20 moduleDeclaration
  pure $ unlines (header : imports <> decls)

main :: IO ()
main = do
  g0 <- initStdGen
  g1 <- runRandomAndPrint moduleHeader g0
  g2 <- foldM (\g _i -> runRandomAndPrint importLine g) g1 ([0 .. 20] :: [Int])
  _g3 <- foldM (\g i -> hPutStrLn stderr (show i) >> runRandomAndPrint moduleDeclaration g) g2 ([0 .. 100] :: [Int])
  pure ()
  where
    runRandomAndPrint :: Random g String -> g -> IO g
    runRandomAndPrint rand g = do
      let (string, g') = runRandom rand g
      putStrLn string
      pure g'

----------

dquotes :: String -> String
dquotes = surround "\"" "\""

parens :: String -> String
parens = surround "(" ")"

braces :: String -> String
braces = surround "{" "}"

brackets :: String -> String
brackets = surround "[" "]"

surround :: String -> String -> String -> String
surround begin end s = begin <> s <> end

commaSep :: [String] -> String
commaSep = sep ", "

sep :: String -> [String] -> String
sep = intercalate

type Random g a = State.State g a

runRandom :: Random g a -> g -> (a, g)
runRandom = State.runState

bool :: RandomGen g => Random g Bool
bool = choose [True, False]

optional :: RandomGen g => Random g a -> Random g (Maybe a)
optional rand = chooseRandom [Just <$> rand, pure Nothing]

chooseRandom :: RandomGen g => NonEmpty (Random g a) -> Random g a
chooseRandom = join . choose

choose :: RandomGen g => NonEmpty a -> Random g a
choose xs = do
  i <- int (0, length xs - 1)
  pure (xs NonEmpty.!! i)

list :: RandomGen g => (Int, Int) -> Random g a -> Random g [a]
list range rand = do
  len <- int range
  replicateM len rand

int :: RandomGen g => (Int, Int) -> Random g Int
int (min', max') = do
  g <- State.get
  let (i, g') = System.Random.uniformR (min', max') g
  State.put g'
  pure i

notKeyword :: RandomGen g => Random g String -> Random g String
notKeyword rand = do
  s <- rand
  if s `elem` keywords then notKeyword rand else pure s

keywords :: [String]
keywords =
  [ "module",
    "exports",
    "import",
    "as",
    "type",
    "foreign",
    "fn",
    "if",
    "then",
    "else",
    "match",
    "with",
    "end",
    "do",
    "return"
  ]

{-

# develop
ghcid --command='stack repl cst-generator' --run

# test
stack run cst-generator | tee debug.ditto | ditto fmt --stdin

# debug
ditto fmt debug.ditto 2>&1 | less -S

-}
