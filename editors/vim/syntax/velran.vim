" Vim syntax file for Velran
" Keep public language tokens in sync with language-core/compiler source syntax.
if exists("b:current_syntax") | finish | endif

syn keyword velranDecl mod model enum object form route type permission critical webhook integration security event
syn keyword velranFn fn page action query component layout
syn keyword velranFlow let set while if return with resource transaction authorize canonical for
syn keyword velranRoute GET POST PUT PATCH DELETE query json upload validate auth rate cache invalidate owner role public user mfa ttl to uses
syn keyword velranPolicy audit idempotency egress verified by signatureHeader timestampHeader replayWindow mutates scoped before until fail flash
syn keyword velranBuiltin sin cos sqrt abs ln log10 log exp pow round floor ceil monotonicNanos len slug stringLen trim trimStart trimEnd lower upper contains startsWith endsWith replace split splitBounded substring indexOf lastIndexOf charAt repeat dict containsKey removeKey regexMatch regexReplace regexCaptures passwordHash passwordVerify newSessionToken newPasswordResetToken newCsrfToken tokenHash presentedTokenHash tokenMatches tokenActive signWebhook verifyWebhookSignature encryptUserData decryptUserData redact expose slugify html json redirect
syn keyword velranLiteral true false Committed RolledBack CommitUnknown Changed
syn keyword velranType String Email Url Slug Int F32 Bool Date DateTime Uuid Decimal Image Upload Db Transaction Html Json Redirect PageContext ActionContext PageError DbError Result List Dict Array Void Secret Sensitive
syn keyword velranValidation length range pattern same items required

syn keyword velranTodo TODO FIXME XXX contained
syn region velranComment start="//" end="$" contains=velranTodo
syn region velranString start=/"/ skip=/\\"/ end=/"/
syn match velranNumber /\<-\=\d\+\(\.\d\+f32\)\?\>/
syn match velranTemplate /{{\s*[^}][^}]*\s*}}/
syn match velranRouteHelper /@\(action\|href\)([^)]*)/
syn match velranOperator /=>\|->\|==\|!=\|<=\|>=\|[+*\/=<>?-]/

hi def link velranDecl Statement
hi def link velranFn Keyword
hi def link velranFlow Keyword
hi def link velranRoute Special
hi def link velranPolicy Special
hi def link velranBuiltin Function
hi def link velranLiteral Boolean
hi def link velranType Type
hi def link velranValidation Special
hi def link velranComment Comment
hi def link velranTodo Todo
hi def link velranString String
hi def link velranNumber Number
hi def link velranTemplate Identifier
hi def link velranRouteHelper Function
hi def link velranOperator Operator

let b:current_syntax = "velran"
