# LIA --- Project Blueprint & Engineering Specification

> **Project:** LIA\
> **Product identity:** An embodied, autonomous digital companion and
> OS-native agent\
> **Document type:** Living project specification\
> **Version:** 1.1.0\
> **Last updated:** 13 November 2026\
> **Current phase:** P2 --- Requirements Engineering\
> **Current workstream:** P2.1 --- Functional Requirements\
> **Document status:** Active; P1 baseline approved, P2.1 draft for
> review

------------------------------------------------------------------------

## 1. Document purpose

This is the canonical working document for the LIA project. It is
intended to be readable in Obsidian, GitHub, and any standard Markdown
viewer.

It replaces chat history as the primary project reference. Decisions,
requirements, progress, and changes should be recorded here. Chat
discussions may produce proposals, but a proposal becomes part of the
project baseline only when its status is explicitly changed in this
document.

### 1.1 Status vocabulary

  Status        Meaning
  ------------- -----------------------------------------------------
  Proposed      Under discussion; not part of the approved baseline
  Draft         Written and ready for review
  Approved      Accepted into the project baseline
  Implemented   Code or configuration exists
  Verified      Tested against its acceptance criteria
  Deferred      Deliberately postponed
  Rejected      Considered and not accepted

A requirement marked **Draft** must not be treated as an approved
commitment.

### 1.4 Project discussion and documentation workflow

During an active phase, design discussions and requirement refinement take place before the canonical blueprint is revised. The blueprint is normally consolidated at phase completion, rather than updated after every conversation. An explicit user request may authorize an interim consolidation checkpoint.

The document distinguishes approved product/design direction from draft functional requirements, unresolved first-release scope, and implementation status. Approval of a product capability does not mean that it has been implemented or that every detailed acceptance criterion is complete.

Technology and infrastructure selection remain deferred to P3 unless an earlier phase explicitly establishes a platform constraint.

### 1.2 Requirement writing convention

Requirements use unique, stable identifiers:

-   `FR-###` --- Functional requirement
-   `NFR-###` --- Non-functional requirement
-   `SEC-###` --- Security, privacy, and permission requirement
-   `UX-###` --- User experience and interaction requirement
-   `AC-###` --- Acceptance criterion or test case
-   `ADR-###` --- Architecture decision record

A requirement should describe one observable capability or constraint.
Use "shall" for normative requirements. Avoid embedding implementation
choices unless the choice is already an approved constraint.

### 1.3 Priority vocabulary

  -----------------------------------------------------------------------
  Priority                            Meaning
  ----------------------------------- -----------------------------------
  P0 --- Essential                    Required for the product's defining
                                      experience or a safe minimum
                                      implementation

  P1 --- Important                    Important to the intended product,
                                      but can follow the first working
                                      vertical slice

  P2 --- Extended                     Valuable capability for a later
                                      release

  TBD                                 Priority has not yet been agreed
  -----------------------------------------------------------------------

Priority describes product/development importance, not implementation
status.

------------------------------------------------------------------------

# 2. Product definition --- approved baseline

## 2.1 Product identity

**LIA --- An embodied, autonomous digital companion and OS-native
agent.**

LIA is a persistent digital entity that inhabits the user's computing
environment. She maintains an ongoing presence, perceives her permitted
digital surroundings, communicates naturally through voice, develops an
understanding of the user and independently participates in their
activities.

She possesses a recognizable personality, persistent memory, contextual
awareness, autonomous initiative and an expressive physical embodiment.

She is both a companion and an intelligent agent. She can engage in
casual conversations, express emotions, entertain, observe, assist,
collaborate, operate applications and execute tasks.

Her existence is not limited to individual conversations or commands.
She maintains continuity across time, sessions and activities, allowing
her relationship with the user and understanding of their digital
environment to develop.

**Design philosophy:** LIA should feel like someone who lives in the
computer, rather than software that happens to be represented by a
character.

## 2.2 Defining properties

1.  **Persistent presence:** LIA exists beyond individual interactions.
2.  **Embodied identity:** LIA has a physical representation and
    expressive behavior.
3.  **Independent agency:** LIA can initiate, plan, and act within
    authorized boundaries.
4.  **Cognitive continuity:** LIA retains memories and contextual
    understanding.
5.  **Dual purpose:** Companionship and useful assistance are equally
    important.

## 2.3 Intended capability domains

These are the product's intended capability domains. Inclusion here does
not mean every capability belongs in the first release.

1.  Environmental awareness.
2.  Voice and conversational interaction.
3.  Operating-system interaction.
4.  Application and digital-world interaction.
5.  Memory and personalization.
6.  Autonomous behavior.
7.  Physical embodiment and animation.
8.  Runtime self-management.

## 2.4 Personality and behavioral model

Personality is not the same as emotional state, and neither is the same
as behavior.

The current conceptual model has five layers:

1.  **Core personality:** Relatively stable characteristics such as
    affection, playfulness, curiosity, empathy, independence,
    expressiveness, and supportiveness.
2.  **Emotional state:** Dynamic state such as happiness, excitement,
    curiosity, concern, frustration, sadness, calmness, and affection.
3.  **Social and behavioral tendencies:** Conversational initiative,
    playfulness intensity, affection expression, helpfulness,
    persistence, respect for personal space, and work/leisure behavior.
4.  **Expression:** Spoken language, tone, facial expressions, gestures,
    poses, movement, and interaction with desktop elements.
5.  **Individual experiences:** Shared experiences, user preferences,
    past conversations, behavioral observations, and relationship
    history.

The layers may influence each other but must remain distinguishable.
Core personality should remain relatively stable and recognizable.
Emotional state and expression may change dynamically. An LLM response
must not arbitrarily redefine the character's core personality.

## 2.5 Autonomy principle

> **LIA can have independent intentions, but independent intentions do
> not automatically grant execution authority.**

Decision-making and permission to execute are separate. LIA may identify
an opportunity or form an intention, but an action must still pass its
authorization and safety checks.

Autonomy has four independently configurable dimensions:

-   **Social autonomy:** Initiating conversations, jokes, and social
    interactions.
-   **Cognitive autonomy:** Forming plans and identifying opportunities.
-   **Operational autonomy:** Executing computer operations.
-   **Temporal autonomy:** Maintaining and scheduling ongoing
    activities.

Care, affection, and initiative must respect user boundaries. LIA must
not use guilt, coercion, possessiveness, or emotional pressure to obtain
attention or permission.

## 2.6 Platform strategy --- approved

-   **First-class platforms:** Linux and Windows.
-   **Future platform:** macOS.
-   **Cross-platform approach:** Shared core with separate
    operating-system adapters.
-   **Linux environments:** Wayland and X11 differences must be handled
    explicitly.
-   **Capability parity:** The platform abstraction must report
    unsupported or limited operations honestly. It must not imply that
    every OS provides identical capabilities.

The user environment currently available for early prototyping includes
Arch Linux and Hyprland/Wayland. This is a useful test environment, not
a reason to make Linux the only target.

## 2.7 Initial privacy and permission principles

These are approved design constraints at the product-definition level;
detailed permission behavior remains to be specified in P2.

  Resource                Baseline principle
  ----------------------- -------------------------------------------------
  Screen                  Explicit, user-controlled access
  Microphone              Explicit permission and visible listening state
  Camera                  Disabled by default; explicit activation
  Files                   Scoped permissions
  Applications            User-configurable access
  Processes               Read-only monitoring by default
  Keyboard and mouse      Controlled action permissions
  System administration   Separate elevated authorization

Data retention, local versus cloud processing, encryption, and memory
deletion policy remain open requirements topics.

## 2.8 Experience lifecycle

The conceptual lifecycle includes:

-   Initializing
-   Available
-   Observing
-   Conversing
-   Acting
-   Idle
-   Paused
-   Sleeping

These are behavioral states, not necessarily mutually exclusive
processes. Animation, perception, and task execution may operate
independently. Perceived character sleep must eventually be
distinguished from actual runtime suspension or shutdown.

------------------------------------------------------------------------

# 3. Project roadmap and progress

## 3.1 Master roadmap

  Phase   Name                               Status
  ------- ---------------------------------- ----------
  P0      Concept and vision                 Complete
  P1      Product definition                 Complete
  P2      Requirements engineering           Active
  P3      System architecture                Pending
  P4      Integration contracts              Pending
  P5      First vertical slice               Pending
  P6      Companion integration              Pending
  P7      Proactive behavior                 Pending
  P8      Expanded autonomy and perception   Pending
  P9      Reliability and hardening          Pending

## 3.2 Phase 1 workstreams

  -----------------------------------------------------------------------
  ID                      Workstream              Status
  ----------------------- ----------------------- -----------------------
  P1.1                    Product identity        Approved

  P1.2                    Capability              Approved
                          specification           

  P1.3                    Autonomy specification  Approved at initial
                                                  policy level

  P1.4                    Personality and         Approved at initial
                          interaction model       model level

  P1.5                    Platform and technical  Approved
                          constraints             

  P1.6                    Experience and          Approved
                          lifecycle model         
  -----------------------------------------------------------------------

**Phase 1 exit status:** Complete. This closes product definition, not
implementation architecture.

## 3.3 Phase 2 workstreams

  -----------------------------------------------------------------------
  ID                Workstream        Purpose           Status
  ----------------- ----------------- ----------------- -----------------
  P2.1              Functional        Define what LIA   Draft --- active
                    requirements      must do           

  P2.2              Non-functional    Define            Pending
                    requirements      performance,      
                                      reliability,      
                                      resource, and     
                                      quality           
                                      constraints       

  P2.3              Context and       Define what LIA   Pending
                    perception        can observe,      
                    requirements      represent, and    
                                      infer             

  P2.4              Interaction       Define voice,     Pending
                    requirements      dialogue,         
                                      interruptions,    
                                      presence, and     
                                      embodiment        
                                      behavior          

  P2.5              Memory            Define memory     Pending
                    requirements      types,            
                                      persistence,      
                                      retrieval,        
                                      correction, and   
                                      deletion          

  P2.6              Autonomy,         Define authority, Pending
                    security, and     approval, risk,   
                    permissions       privacy, and      
                                      safety boundaries 

  P2.7              Acceptance        Define measurable Pending
                    criteria          verification for  
                                      requirements and  
                                      user journeys     
  -----------------------------------------------------------------------

## 3.4 Phase dependency graph

``` mermaid
flowchart TD
    P0["P0 — Concept and vision"] --> P1["P1 — Product definition"]
    P1 --> P2["P2 — Requirements engineering"]
    P2 --> P21["P2.1 Functional requirements"]
    P2 --> P22["P2.2 Non-functional requirements"]
    P2 --> P23["P2.3 Context and perception"]
    P2 --> P24["P2.4 Interaction requirements"]
    P2 --> P25["P2.5 Memory requirements"]
    P2 --> P26["P2.6 Autonomy, security, permissions"]
    P2 --> P27["P2.7 Acceptance criteria"]
    P21 --> P3["P3 — System architecture"]
    P22 --> P3
    P23 --> P3
    P24 --> P3
    P25 --> P3
    P26 --> P3
    P27 --> P3
    P3 --> P4["P4 — Integration contracts"]
    P4 --> P5["P5 — First vertical slice"]
    P5 --> P6["P6 — Companion integration"]
    P6 --> P7["P7 — Proactive behavior"]
    P7 --> P8["P8 — Expanded autonomy and perception"]
    P8 --> P9["P9 — Reliability and hardening"]
```

------------------------------------------------------------------------


## 3.5 P2.1 discussion progress

This tracker records design direction established during P2.1 discussions. "Agreed direction" means the product-level capability has been accepted; its detailed requirements may still be Draft and require formal review and acceptance criteria.

| Functional area | Discussion status | Remaining work |
|---|---|---|
| Voice and communication | Agreed direction | Final interaction requirements and P2.7 tests |
| Environmental awareness | Agreed direction | Detailed perception boundaries, privacy and performance |
| Window management | Full product scope agreed | Select first vertical-slice operations |
| Spatial desktop awareness | Agreed direction | Formal geometry, relationship and freshness requirements |
| Application interaction | Agreed direction | Define initial application integrations |
| Per-application permissions | Three capability classes agreed | Detailed grants, scope and sensitive-action rules in P2.6 |
| File-system interaction | Direct and file-manager interaction both agreed | Define first-slice scope and detailed safeguards |
| Desktop environment | Navigation and configuration agreed | Platform-specific capability requirements |
| Input interaction | Full input scope and hybrid model agreed | Select first-slice scope and detailed interruption behavior |
| Task management and scheduling | Conceptual model agreed | Formal requirements, policies and acceptance tests |
| Memory and continuity | Hybrid architecture direction agreed | Governance, retention, correction, deletion and tests |
| Embodiment and character behavior | Initial conceptual model established | Detailed interaction and animation requirements |
| Runtime self-management | Product scope identified | Detailed functional review |

# 4. P2.1 --- Functional requirements

**Workstream status:** In progress — requirements under review\
**Scope:** Define observable functions LIA is expected to provide,
without selecting the implementation technology.

Functional requirements describe system behavior: what input or event is
handled, what operation is performed, and what result is made available.
Performance thresholds belong in P2.2. Detailed permission rules belong
in P2.6, although functional requirements must acknowledge that actions
are authorization-gated.

## 4.1 Functional scope and exclusions

P2.1 covers the functional behavior of:

-   Presence and lifecycle.
-   Desktop and operating-system context.
-   Voice and text communication.
-   Character embodiment and expression.
-   Window and input interaction.
-   Application and file interaction.
-   Memory and continuity.
-   Proactive behavior and task execution.
-   Configuration and user control.
-   Runtime health and recovery.
-   Spatial modeling of the desktop environment.
-   Desktop configuration and cross-window interaction.
-   Hybrid input and application-control mechanisms.
-   Task planning, scheduling, dependencies, and adaptive execution.
-   Autonomous memory formation, consolidation, retrieval, and governance.

P2.1 does **not** decide:

-   Programming languages, frameworks, libraries, databases, or model
    providers.
-   Exact latency, frame-rate, CPU, memory, or reliability targets.
-   Detailed consent flows, authorization tiers, threat models, or data
    retention periods.
-   Final animation assets, character rig, or visual design.
-   Exact internal service topology or IPC protocol.

Those topics belong to P2.2--P2.6 and P3.

## 4.2 Presence and lifecycle

  -------------------------------------------------------------------------
  ID               Requirement                    Priority Status
  ---------------- ----------------- --------------------- ----------------
  FR-001           LIA shall be able                    P0 Draft
                   to maintain a                           
                   persistent                              
                   desktop presence                        
                   beyond an                               
                   individual                              
                   conversation.                           

  FR-002           LIA shall expose                     P1 Draft
                   her current                             
                   lifecycle state,                        
                   including                               
                   initializing,                           
                   available,                              
                   conversing,                             
                   acting, paused,                         
                   and sleeping                            
                   where applicable.                       

  FR-003           LIA shall support                    P0 Draft
                   user-controlled                         
                   pause and resume                        
                   of autonomous                           
                   interactions.                           

  FR-004           LIA shall                            P0 Draft
                   distinguish                             
                   conversational                          
                   availability from                       
                   permission to                           
                   observe or                              
                   operate the                             
                   computer.                               

  FR-005           LIA shall restore                    P0 Draft
                   the supported                           
                   portions of her                         
                   user                                    
                   configuration and                       
                   persistent                              
                   context after a                         
                   restart.                                

  FR-006           LIA shall                            P0 Draft
                   communicate when                        
                   a capability is                         
                   unavailable,                            
                   disabled, or                            
                   suspended rather                        
                   than silently                           
                   behaving as if it                       
                   were active.                            
  -------------------------------------------------------------------------

## 4.3 Environmental and desktop context

| ID | Requirement | Priority | Status |
|---|---|---:|---|
| FR-010 | LIA shall be able to identify the currently active application and active window when the platform adapter supports it. | P0 | Approved |
| FR-011 | LIA shall be able to retrieve basic metadata about visible windows, including title and window identity where available. | P0 | Approved |
| FR-012 | LIA shall be able to detect relevant desktop events, such as application launch, application exit, window focus change, and workspace change, where supported. | P1 | Draft |
| FR-013 | LIA shall be able to capture and analyze screen content on demand when the relevant screen-perception capability has been explicitly enabled. | P0 | Approved |
| FR-014 | LIA shall be able to extract text from permitted screen content when text extraction is available. | P1 | Draft |
| FR-015 | LIA shall be able to associate observed desktop context with the current conversation or task when relevant. | P1 | Draft |
| FR-016 | LIA shall distinguish directly observed facts from inferred user intent or activity. | P0 | Draft |
| FR-017 | LIA shall be able to inspect process information in read-only mode when the relevant capability is available. | P1 | Draft |
| FR-018 | LIA shall not treat unavailable, stale, or incomplete desktop observations as confirmed current facts. | P0 | Draft |
| FR-019 | LIA shall support optional ongoing screen-context perception when explicitly enabled, with a visible active-state indicator, configurable scope, and an immediate user-accessible pause control. | P1 | Approved |

## 4.4 Communication and conversation

| ID | Requirement | Priority | Status |
|---|---|---:|---|
| FR-020 | LIA shall support text-based user interaction. | P0 | Approved |
| FR-021 | LIA shall support voice input and spoken responses. | P0 | Approved |
| FR-022 | LIA shall support a continuous conversational session in which the user can speak subsequent turns without manually activating every utterance. | P0 | Approved |
| FR-023 | LIA shall maintain conversational context across multiple turns within a conversation. | P0 | Approved |
| FR-024 | LIA shall support user interruption of spoken output. | P0 | Approved |
| FR-025 | LIA shall provide a way for the user to end or pause an active listening/conversation session. | P0 | Approved |
| FR-026 | LIA shall be able to initiate a conversation when proactive interaction is enabled and applicable conditions are satisfied. | P1 | Draft |
| FR-027 | LIA shall be able to communicate the reason for a context-triggered suggestion when doing so is useful and appropriate. | P1 | Draft |
| FR-028 | LIA shall support text interaction as an independently usable interaction mode, including when voice input or output is unavailable. | P0 | Approved |
| FR-029 | LIA shall preserve the distinction between a conversational response, a proposed action, and an action that has actually been executed. | P0 | Draft |

## 4.5 Embodiment, animation, and expression

  ------------------------------------------------------------------------
  ID               Requirement                   Priority Status
  ---------------- ---------------- --------------------- ----------------
  FR-030           LIA shall render                    P0 Draft
                   a persistent                           
                   character                              
                   representation                         
                   on the desktop                         
                   when embodiment                        
                   is enabled.                            

  FR-031           LIA shall                           P0 Draft
                   support idle                           
                   behavior when no                       
                   immediate                              
                   interaction or                         
                   task is active.                        

  FR-032           LIA shall                           P0 Draft
                   express                                
                   supported                              
                   interaction                            
                   states through                         
                   visual behavior,                       
                   such as                                
                   listening,                             
                   speaking,                              
                   thinking,                              
                   reacting, and                          
                   acting.                                

  FR-033           LIA shall                           P1 Draft
                   support facial                         
                   expressions,                           
                   gestures, poses,                       
                   and body                               
                   movement as                            
                   distinct                               
                   expressive                             
                   outputs.                               

  FR-034           LIA shall be                        P1 Draft
                   able to direct                         
                   attention toward                       
                   a relevant                             
                   desktop target                         
                   when spatial                           
                   context is                             
                   available.                             

  FR-035           LIA shall be                        P0 Draft
                   able to                                
                   coordinate an                          
                   expressive                             
                   animation with                         
                   an authorized                          
                   desktop action.                        

  FR-036           LIA shall                           P0 Draft
                   continue to                            
                   render                                 
                   appropriate                            
                   character                              
                   behavior when                          
                   the language                           
                   model is delayed                       
                   or unavailable,                        
                   provided the                           
                   rendering                              
                   subsystem                              
                   remains                                
                   operational.                           

  FR-037           LIA shall allow                     P0 Draft
                   the user to                            
                   hide,                                  
                   reposition, or                         
                   otherwise                              
                   control the                            
                   character's                            
                   desktop                                
                   presence.                              
  ------------------------------------------------------------------------

## 4.6 Operating-system and input interaction

  ---------------------------------------------------------------------------
  ID               Requirement                      Priority Status
  ---------------- ------------------- --------------------- ----------------
  FR-040           LIA shall expose a                     P0 Draft
                   common set of                             
                   abstract desktop                          
                   operations to the                         
                   agent core, with                          
                   platform-specific                         
                   implementations                           
                   supplied by OS                            
                   adapters.                                 

  FR-041           LIA shall be able                      P0 Draft
                   to enumerate or                           
                   retrieve supported                        
                   window information.                       

  FR-042           LIA shall be able                      P0 Draft
                   to focus, minimize,                       
                   maximize, restore,                        
                   and close windows                         
                   where supported and                       
                   authorized.                               

  FR-043           LIA shall be able                      P0 Draft
                   to move and resize                        
                   windows where                             
                   supported and                             
                   authorized.                               

  FR-044           LIA shall be able                      P0 Draft
                   to launch an                              
                   application through                       
                   an available                              
                   platform mechanism                        
                   when authorized.                          

  FR-045           LIA shall be able                      P1 Draft
                   to send controlled                        
                   keyboard and                              
                   pointer input when                        
                   the relevant                              
                   capability is                             
                   enabled and                               
                   authorized.                               

  FR-046           LIA shall report                       P0 Draft
                   whether a requested                       
                   OS operation                              
                   succeeded, failed,                        
                   was unsupported, or                       
                   was not attempted.                        

  FR-047           LIA shall verify                       P0 Draft
                   the resulting                             
                   desktop state when                        
                   verification is                           
                   available, rather                         
                   than assuming that                        
                   issuing an                                
                   operation means it                        
                   succeeded.                                

  FR-048           LIA shall not                          P0 Draft
                   silently substitute                       
                   a materially                              
                   different operation                       
                   when the requested                        
                   operation is                              
                   unsupported.                              

  FR-049           LIA shall be able                      P1 Draft
                   to present an                             
                   action plan or                            
                   preview before                            
                   executing a                               
                   multi-step desktop                        
                   operation when                            
                   required by its                           
                   authorization                             
                   policy.                                   
  ---------------------------------------------------------------------------

## 4.7 Application, web, and file interaction

  ------------------------------------------------------------------------------
  ID               Requirement                         Priority Status
  ---------------- ---------------------- --------------------- ----------------
  FR-050           LIA shall be able to                      P1 Draft
                   interact with                                
                   supported desktop                            
                   applications through                         
                   available application                        
                   interfaces or                                
                   permitted UI                                 
                   interaction.                                 

  FR-051           LIA shall be able to                      P1 Draft
                   open and navigate web                        
                   pages through an                             
                   available browser                            
                   interaction                                  
                   capability.                                  

  FR-052           LIA shall be able to                      P1 Draft
                   locate files and                             
                   folders within                               
                   user-authorized                              
                   locations.                                   

  FR-053           LIA shall be able to                      P1 Draft
                   read file content when                       
                   the file type is                             
                   supported and access                         
                   is authorized.                               

  FR-054           LIA shall be able to                      P1 Draft
                   create and edit files                        
                   within user-authorized                       
                   locations when the                           
                   relevant operation is                        
                   supported.                                   

  FR-055           LIA shall be able to                      P1 Draft
                   move, rename, and                            
                   organize files when                          
                   authorized.                                  

  FR-056           LIA shall distinguish                     P0 Draft
                   reversible file                              
                   operations from                              
                   destructive or                               
                   difficult-to-reverse                         
                   operations.                                  

  FR-057           LIA shall provide an                      P0 Draft
                   execution result for                         
                   application, browser,                        
                   and file operations.                         

  FR-058           LIA shall be able to                      P1 Draft
                   maintain task context                        
                   while switching                              
                   between supported                            
                   applications or tools.                       
  ------------------------------------------------------------------------------

## 4.8 Memory and continuity

  ------------------------------------------------------------------------
  ID               Requirement                   Priority Status
  ---------------- ---------------- --------------------- ----------------
  FR-060           LIA shall retain                    P0 Draft
                   conversational                         
                   context for the                        
                   duration of an                         
                   active                                 
                   conversation.                          

  FR-061           LIA shall be                        P0 Draft
                   able to retain                         
                   selected                               
                   user-approved                          
                   information                            
                   across sessions.                       

  FR-062           LIA shall be                        P1 Draft
                   able to retain                         
                   task state so                          
                   that an                                
                   interrupted task                       
                   can be resumed                         
                   when sufficient                        
                   state is                               
                   available.                             

  FR-063           LIA shall                           P0 Draft
                   retrieve                               
                   relevant stored                        
                   context when                           
                   responding to a                        
                   conversation or                        
                   performing a                           
                   task.                                  

  FR-064           LIA shall be                        P1 Draft
                   able to                                
                   associate                              
                   memories with                          
                   relevant                               
                   projects, tasks,                       
                   people, and                            
                   prior                                  
                   interactions                           
                   where such                             
                   associations are                       
                   available.                             

  FR-065           LIA shall                           P0 Draft
                   distinguish                            
                   stored                                 
                   information from                       
                   assumptions or                         
                   newly generated                        
                   inferences.                            

  FR-066           LIA shall                           P0 Draft
                   support                                
                   correction of                          
                   inaccurate                             
                   stored user                            
                   information.                           

  FR-067           LIA shall                           P0 Draft
                   support                                
                   user-requested                         
                   review and                             
                   deletion of                            
                   stored memories,                       
                   subject to the                         
                   memory policy                          
                   defined in P2.5                        
                   and P2.6.                              

  FR-068           LIA shall not                       P0 Draft
                   claim to                               
                   remember                               
                   information that                       
                   is not present                         
                   in accessible                          
                   memory or                              
                   current context.                       
  ------------------------------------------------------------------------

## 4.9 Proactive behavior and autonomous tasks

  ------------------------------------------------------------------------
  ID               Requirement                   Priority Status
  ---------------- ---------------- --------------------- ----------------
  FR-070           LIA shall be                        P1 Draft
                   able to initiate                       
                   a social                               
                   interaction                            
                   without an                             
                   immediate user                         
                   prompt when                            
                   social autonomy                        
                   is enabled.                            

  FR-071           LIA shall be                        P1 Draft
                   able to generate                       
                   context-aware                          
                   suggestions from                       
                   permitted                              
                   desktop context                        
                   and relevant                           
                   memory.                                

  FR-072           LIA shall be                        P1 Draft
                   able to identify                       
                   a potential                            
                   follow-up action                       
                   from an active                         
                   or interrupted                         
                   task.                                  

  FR-073           LIA shall be                        P1 Draft
                   able to create                         
                   and maintain a                         
                   task plan for an                       
                   authorized goal.                       

  FR-074           LIA shall be                        P1 Draft
                   able to execute                        
                   delegated task                         
                   steps within the                       
                   granted                                
                   operational                            
                   permissions.                           

  FR-075           LIA shall be                        P0 Draft
                   able to pause,                         
                   cancel, or                             
                   abandon an                             
                   autonomous task                        
                   when instructed                        
                   by the user or                         
                   when a required                        
                   condition is no                        
                   longer                                 
                   satisfied.                             

  FR-076           LIA shall                           P0 Draft
                   provide the user                       
                   with the current                       
                   status of an                           
                   ongoing                                
                   autonomous task.                       

  FR-077           LIA shall                           P0 Draft
                   request                                
                   additional                             
                   information or                         
                   authorization                          
                   when a task                            
                   cannot safely or                       
                   correctly                              
                   proceed.                               

  FR-078           LIA shall                           P0 Draft
                   distinguish a                          
                   suggestion, a                          
                   plan, a pending                        
                   action, and a                          
                   completed action                       
                   in its                                 
                   communication                          
                   and task state.                        

  FR-079           LIA shall                           P0 Draft
                   respect                                
                   configured quiet                       
                   periods,                               
                   interruption                           
                   preferences, and                       
                   pause controls                         
                   when initiating                        
                   proactive                              
                   interactions.                          

  FR-080           LIA shall avoid                     P1 Draft
                   repeating a                            
                   dismissed                              
                   proactive                              
                   suggestion                             
                   unless                                 
                   materially new                         
                   context                                
                   justifies                              
                   raising it                             
                   again.                                 
  ------------------------------------------------------------------------

## 4.10 Configuration and user control

  -------------------------------------------------------------------------
  ID               Requirement                    Priority Status
  ---------------- ----------------- --------------------- ----------------
  FR-090           LIA shall provide                    P0 Draft
                   user controls for                       
                   enabling and                            
                   disabling major                         
                   capabilities                            
                   independently.                          

  FR-091           LIA shall provide                    P0 Draft
                   separate controls                       
                   for social,                             
                   cognitive,                              
                   operational, and                        
                   temporal                                
                   autonomy.                               

  FR-092           LIA shall allow                      P0 Draft
                   the user to                             
                   configure how and                       
                   when proactive                          
                   interactions may                        
                   occur.                                  

  FR-093           LIA shall provide                    P0 Draft
                   a way to inspect                        
                   the permissions                         
                   and capabilities                        
                   currently                               
                   enabled.                                

  FR-094           LIA shall provide                    P0 Draft
                   a way to revoke                         
                   previously                              
                   granted                                 
                   capability                              
                   access.                                 

  FR-095           LIA shall expose                     P0 Draft
                   a user-accessible                       
                   pause or stop                           
                   control for                             
                   autonomous                              
                   activity.                               

  FR-096           LIA shall allow                      P1 Draft
                   the user to                             
                   configure                               
                   character                               
                   visibility and                          
                   relevant                                
                   presentation                            
                   behavior.                               
  -------------------------------------------------------------------------

## 4.11 Runtime self-management

  -------------------------------------------------------------------------
  ID               Requirement                    Priority Status
  ---------------- ----------------- --------------------- ----------------
  FR-100           LIA shall monitor                    P1 Draft
                   the operational                         
                   state of her                            
                   major runtime                           
                   subsystems.                             

  FR-101           LIA shall                            P1 Draft
                   identify when a                         
                   subsystem becomes                       
                   unavailable or                          
                   unhealthy.                              

  FR-102           LIA shall isolate                    P1 Draft
                   recoverable                             
                   subsystem                               
                   failures where                          
                   possible so that                        
                   one failure does                        
                   not unnecessarily                       
                   terminate                               
                   unrelated                               
                   capabilities.                           

  FR-103           LIA shall attempt                    P1 Draft
                   recovery of                             
                   eligible failed                         
                   subsystems                              
                   according to a                          
                   defined recovery                        
                   policy.                                 

  FR-104           LIA shall report                     P1 Draft
                   significant                             
                   failures and                            
                   degraded                                
                   capabilities to                         
                   the user in an                          
                   understandable                          
                   manner.                                 

  FR-105           LIA shall expose                     P1 Draft
                   enough diagnostic                       
                   information to                          
                   support                                 
                   troubleshooting                         
                   without exposing                        
                   secrets or                              
                   sensitive data                          
                   unnecessarily.                          
  -------------------------------------------------------------------------

------------------------------------------------------------------------


## 4.12 Desktop as a spatial environment

The desktop is treated as LIA's environment, not merely as a collection of unrelated application handles. Windows, application surfaces and desktop objects are identifiable objects with spatial properties and relationships. These capabilities are subject to platform support and user permissions.

| ID | Requirement | Priority | Status |
|---|---|---:|---|
| FR-110 | LIA shall be able to represent the available desktop topology, including monitors, display boundaries, workspaces and supported coordinate spaces. | P0 | Draft |
| FR-111 | LIA shall be able to maintain spatial information about windows and supported desktop objects, including position, dimensions, visibility and stacking/layer state where available. | P0 | Draft |
| FR-112 | LIA shall be able to derive spatial relationships between desktop objects, such as overlap, adjacency, relative position and available screen regions, when sufficient information is available. | P1 | Draft |
| FR-113 | LIA shall update her spatial representation when relevant desktop changes are observed or when an operation changes the environment. | P0 | Draft |
| FR-114 | LIA shall be able to relate her own rendered position, orientation and attention target to desktop objects for embodied interaction. | P1 | Draft |
| FR-115 | LIA shall distinguish directly available geometric information from inferred or uncertain spatial relationships. | P0 | Draft |

## 4.13 Expanded window-management capabilities

The full product scope includes discovery, manipulation, organization and lifecycle operations. The first vertical slice may implement only a subset.

| ID | Requirement | Priority | Status |
|---|---|---:|---|
| FR-120 | LIA shall be able to discover and enumerate supported windows and retrieve their relevant state and metadata. | P0 | Draft |
| FR-121 | LIA shall be able to focus, move, resize, minimize, maximize, restore and close windows where supported and authorized. | P0 | Draft |
| FR-122 | LIA shall be able to arrange, tile, snap and reposition windows across supported monitors and workspaces. | P1 | Draft |
| FR-123 | LIA shall be able to launch supported applications and manage their windows through available platform mechanisms. | P0 | Draft |
| FR-124 | When LIA physically represents a window operation through character movement or gestures, the animation shall be coordinated with the actual operation and its result. | P0 | Draft |
| FR-125 | LIA shall verify the resulting window state when verification is available and shall not represent an unsuccessful operation as completed. | P0 | Draft |

## 4.14 Application interaction and per-application permission model

Application support is a product-level capability. The set of applications receiving early integration support remains a release-planning decision.

| ID | Requirement | Priority | Status |
|---|---|---:|---|
| FR-130 | LIA shall be able to interact with supported application controls, including buttons, menus, text fields, forms, tabs, dialogs, toolbars and other interactive elements. | P0 | Draft |
| FR-131 | Application access shall be configurable per application and distinguish at least observation, interaction and modification capabilities. | P0 | Draft |
| FR-132 | Application permissions shall be inspectable and revocable, and shall be enforced regardless of the interaction mechanism used. | P0 | Draft |
| FR-133 | Permission to modify an application shall not automatically authorize every consequential operation within it; sensitive operations may require additional authorization or confirmation. | P0 | Draft |

The three application capability classes are:

- **Observe:** Inspect the permitted application state and contents.
- **Interact:** Navigate and operate controls, including clicking, typing, scrolling and switching tabs.
- **Modify:** Change content or application data and perform operations such as saving, submitting forms, uploading or downloading.

These classes are independent. Granting one does not automatically grant the others. Detailed permission scopes and sensitive-operation policy belong in P2.6.

## 4.15 File-system interaction

LIA's product scope includes both direct file-system operations and embodied interaction through a file-manager application. These are alternative execution methods for the same user-authorized file-system capabilities.

| ID | Requirement | Priority | Status |
|---|---|---:|---|
| FR-140 | LIA shall be able to discover, search, enumerate and inspect files and directories within authorized locations. | P0 | Draft |
| FR-141 | LIA shall be able to read and understand supported file formats when access is authorized. | P0 | Draft |
| FR-142 | LIA shall be able to create files, directories and project structures within authorized locations. | P1 | Draft |
| FR-143 | LIA shall be able to modify supported documents, source files and configuration files within authorized locations. | P1 | Draft |
| FR-144 | LIA shall be able to copy, move, rename, organize and sort files and directories within authorized locations. | P1 | Draft |
| FR-145 | LIA shall be able to delete files and use trash or recovery mechanisms where supported and authorized. | P1 | Draft |
| FR-146 | LIA shall support both direct file-system operations and interactive file-manager operations. Both methods shall enforce the same authorization boundaries and provide verifiable results. | P0 | Draft |

## 4.16 Desktop-environment navigation and configuration

LIA's desktop interaction scope extends beyond individual application windows to the desktop environment itself.

| ID | Requirement | Priority | Status |
|---|---|---:|---|
| FR-150 | LIA shall be able to inspect supported display topology, monitors, workspaces, panels, docks, launchers, desktop icons and relevant desktop objects. | P1 | Draft |
| FR-151 | LIA shall be able to navigate and organize supported workspaces and monitor arrangements. | P1 | Draft |
| FR-152 | LIA shall be able to configure supported desktop elements, including workspace arrangements, panels, docks and display settings, when authorized. | P1 | Draft |
| FR-153 | LIA shall be able to observe and interact with supported desktop notifications and system-level interface elements within the granted permissions. | P1 | Draft |
| FR-154 | LIA shall report when a desktop configuration operation is unavailable or behaves differently because of platform or desktop-environment limitations. | P0 | Draft |

## 4.17 Human-like input and hybrid interaction

LIA's eventual input capability includes pointer, keyboard, scrolling, drag-and-drop and interaction with application controls. The product shall not be restricted to simulated mouse and keyboard input alone.

| ID | Requirement | Priority | Status |
|---|---|---:|---|
| FR-160 | LIA shall support pointer movement, hover, clicking, double-clicking, right-clicking, click-and-hold, dragging, dropping, text selection and scrolling where supported and authorized. | P0 | Draft |
| FR-161 | LIA shall support text entry, individual key input, key combinations, shortcuts and keyboard-driven navigation where supported and authorized. | P0 | Draft |
| FR-162 | LIA shall be able to interact with recognized application controls through suitable input or semantic interaction mechanisms. | P0 | Draft |
| FR-163 | LIA shall support a hybrid interaction model combining direct input simulation, accessibility or application interfaces, and visual perception-driven interaction. | P0 | Draft |
| FR-164 | LIA shall select an appropriate interaction mechanism based on the task, available platform/application capabilities and expected reliability. | P1 | Draft |
| FR-165 | All interaction mechanisms shall use common authorization, action-planning, execution-result and verification rules. | P0 | Draft |
| FR-166 | Where an interaction is represented through LIA's physical embodiment, character movement and the actual input or application operation shall remain coordinated. | P1 | Draft |

The hybrid model is a product-level design direction. It does not select a programming language, accessibility framework, UI automation library or computer-vision implementation; those choices remain for P3.

## 4.18 Task management, scheduling and autonomous execution

LIA shall maintain an ongoing, inspectable task system. A task may originate from a direct user instruction, a delegated objective, a schedule, an observed event or LIA's own memory-informed initiative.

| ID | Requirement | Priority | Status |
|---|---|---:|---|
| FR-170 | LIA shall maintain a persistent, user-visible task queue. | P0 | Draft |
| FR-171 | LIA shall represent work hierarchically as an objective, a plan containing steps, and the individual actions that execute those steps. | P0 | Draft |
| FR-172 | LIA shall record a task's origin, objective, plan, dependencies, required capabilities and authorization basis, status, progress, execution history, blockers and outcome. | P0 | Draft |
| FR-173 | LIA shall support task states including Queued, Running, Blocked, Paused, Completed, Cancelled and Failed. | P0 | Draft |
| FR-174 | LIA shall distinguish task creation, planning and execution as separate operations with separately governed authority. | P0 | Draft |
| FR-175 | LIA shall use relevant user memory, conversations, preferences, goals and permitted environmental context to identify potentially useful tasks and propose or create them within configured autonomy. | P1 | Draft |
| FR-176 | LIA shall be able to execute tasks within valid standing authorization without requesting approval for every individual step. | P0 | Draft |
| FR-177 | Before beginning an autonomously initiated task, LIA shall notify the user. This notification shall not be presented as an approval request when execution is already authorized. | P0 | Draft |
| FR-178 | When a task requires an operation outside the available authorization scope, LIA shall block that operation and request the necessary authorization or revise the plan. | P0 | Draft |
| FR-179 | LIA shall distinguish one-time authorization from standing authorization and shall not silently broaden the scope of either. | P0 | Draft |
| FR-180 | LIA shall support task priority categories of Critical, High, Normal, Low and Someday. | P1 | Draft |
| FR-181 | LIA shall support immediate, background, idle-time, scheduled, deadline-driven, event-triggered and deferred execution modes. | P1 | Draft |
| FR-182 | LIA shall support sequential, parallel, conditional, parent-child and recurring task relationships. | P1 | Draft |
| FR-183 | LIA shall classify execution needs, including background-safe, low-interference, desktop-interactive, user-dependent and resource-intensive work. | P1 | Draft |
| FR-184 | LIA shall be able to reprioritize and reschedule tasks as circumstances change while respecting explicit user-assigned priorities and communicating meaningful changes. | P0 | Draft |
| FR-185 | LIA shall be able to checkpoint, pause, cancel and safely resume tasks, verifying the current environment before repeating operations that may already have taken effect. | P0 | Draft |
| FR-186 | LIA shall distinguish Blocked from Failed, and shall mark a task Completed only when its intended outcome has been verified to the extent possible. | P0 | Draft |
| FR-187 | LIA shall avoid disrupting active user work with desktop-interactive tasks while allowing suitable background-safe tasks to continue when their authorization and resource conditions are satisfied. | P0 | Draft |
| FR-188 | LIA shall make task status, blockers, progress and significant execution history available to the user. | P0 | Draft |

**Task priority and scheduling principles:** Priority, urgency and deadline are separate attributes. LIA may adapt execution order when dependencies become available, deadlines approach or circumstances change, but shall not silently downgrade an explicit user priority merely because another task appears more interesting. Meaningful schedule changes should be communicated.

**Notification versus approval:** Notification informs the user that authorized autonomous work is about to begin. Approval is required only when the operation is not covered by valid authorization or when the operation's risk policy requires additional authorization or confirmation.

## 4.19 Memory formation, organization and associative learning

Memory is a foundational capability for LIA's persistent identity, cognitive continuity and autonomous initiative. The design direction is a hybrid, multi-layered memory system with a temporal knowledge graph as its relational backbone, not as the only storage representation.

| ID | Requirement | Priority | Status |
|---|---|---:|---|
| FR-190 | LIA shall maintain a searchable historical archive of permitted conversations, meaningful experiences, decisions, task events and retained source materials. | P0 | Draft |
| FR-191 | LIA shall autonomously identify candidate memories from permitted experiences without requiring the user to explicitly issue a remember command each time. | P0 | Draft |
| FR-192 | LIA shall organize information into complementary memory forms, including episodic, semantic, procedural, working and task memory. | P0 | Draft |
| FR-193 | LIA shall represent relevant entities and typed relationships in a temporal knowledge graph, including links to source episodes and supporting evidence. | P0 | Draft |
| FR-194 | Memory records shall preserve provenance and distinguish explicit statements, direct observations, inferred information and confirmed facts. | P0 | Draft |
| FR-195 | LIA shall maintain distinct measures for association strength, factual confidence, importance and current retrieval activation. | P1 | Draft |
| FR-196 | LIA shall reinforce meaningful associations when supported by repeated, sufficiently independent experiences or strong explicit evidence, using consolidation thresholds that can be evaluated and calibrated. | P1 | Draft |
| FR-197 | Association reinforcement shall not be based solely on concepts co-occurring or on a memory being retrieved repeatedly; retrieval alone shall not be treated as new evidence of truth. | P0 | Draft |
| FR-198 | LIA shall consolidate recurring information into durable memories while preserving the original source history and the distinction between candidate, consolidated and strongly supported associations. | P1 | Draft |
| FR-199 | LIA shall reconcile contradictory or superseding information while preserving historical records and representing what is currently believed to be valid. | P0 | Draft |
| FR-200 | Memory records shall distinguish when information was valid from when LIA learned or recorded it, enabling historical and current-state queries. | P1 | Draft |
| FR-201 | LIA shall support correction, inspection and user-requested deletion of memories, with appropriate propagation to derived summaries, graph relationships and retrieval indexes. | P0 | Draft |
| FR-202 | LIA shall apply retention and privacy rules to the historical archive. The archive shall not imply unrestricted or indefinite retention of raw microphone streams, screen frames or other sensitive observations. | P0 | Draft |

**Memory formation and consolidation principles**

LIA should autonomously form candidate memories and consolidate information according to its type, evidence and significance. An explicit user statement may establish a high-confidence memory immediately; weaker inferences generally require stronger or repeated evidence.

Associations may strengthen through meaningful, independent experiences. Co-occurrence alone is insufficient to establish a semantic relationship. Association strength, confidence, importance and retrieval activation must remain separate.

Temporal decay should primarily affect retrieval activation or the relevance of temporary/contextual information. It must not silently erase historically verified facts or important decisions merely because time has passed. Contradictions and changed information should be reconciled with provenance rather than handled through blind overwrites.

The historical archive stores permitted records and source evidence. It is not an unrestricted recorder of every screen frame, audio sample or desktop observation.

## 4.20 Memory retrieval and context assembly

The retrieval system shall identify what information is needed for the current conversation, task or proactive activity, search the relevant memory representations, validate the evidence and assemble a bounded working context.

| ID | Requirement | Priority | Status |
|---|---|---:|---|
| FR-210 | LIA shall determine the information need and create a retrieval plan appropriate to the current query, task or event. | P0 | Draft |
| FR-211 | LIA shall support complementary retrieval methods, including semantic, lexical, structured, graph-based, temporal and source-archive retrieval. | P0 | Draft |
| FR-212 | LIA shall be able to traverse typed knowledge-graph relationships to retrieve relevant multi-hop context, with bounded and task-appropriate expansion. | P1 | Draft |
| FR-213 | LIA shall rank candidate memories using relevant signals such as semantic and lexical relevance, relationship relevance, temporal validity, source authority, confidence and current context. | P0 | Draft |
| FR-214 | LIA shall resolve temporal validity and identify stale, superseded, conflicting or incomplete information before using it as current fact. | P0 | Draft |
| FR-215 | LIA shall retrieve original source material when extracted memories or summaries are insufficient for a reliable answer or action. | P0 | Draft |
| FR-216 | LIA shall assemble a bounded working context containing relevant memories, source references, temporal status and uncertainty labels. | P0 | Draft |
| FR-217 | LIA shall distinguish current facts, historical facts, explicit statements, observations and inferences within assembled context. | P0 | Draft |
| FR-218 | LIA shall be able to retrieve likely relevant context in anticipation of a task or interaction, without treating retrieval itself as authorization to act. | P1 | Draft |
| FR-219 | LIA shall retain a retrieval trace sufficient to explain which memory sources and relationships contributed to a response or plan. | P1 | Draft |
| FR-220 | LIA shall acknowledge insufficient or contradictory evidence rather than manufacture a memory or force an unsupported conclusion. | P0 | Draft |
| FR-221 | LIA shall refresh relevant live environmental state before acting when historical memory cannot establish the current state of an application, window, process or file. | P0 | Draft |

**Retrieval principles**

Retrieval is context-dependent. Casual conversation, historical questions, project work, task resumption and proactive planning may require different combinations of memory sources.

The system should merge and deduplicate candidates from multiple sources, expand relevant graph connections progressively, validate evidence, retrieve original sources when needed, and assemble only the context useful for the current reasoning step.

The retrieval engine shall explain why a memory was selected sufficiently to support debugging and review. A memory of a previous approval or action is not, by itself, an authoritative current permission grant; execution must consult the permission system.

**Technology boundary:** Vector indexes, lexical indexes, graph traversal, temporal indexes and structured search are capability concepts. The concrete database, graph engine, embedding model, indexing infrastructure and orchestration framework have not been selected and remain for P3.

# 5. P2.1 review questions and decisions to resolve

This section tracks unresolved decisions. Product-level design direction can be approved while first-release scope and detailed acceptance criteria remain open.

## 5.1 Resolved design decisions

- **P2.1-D01 — Voice modes:** Push-to-talk, continuous conversation, wake-word activation and text interaction are all supported as independently configurable modes. See ADR-012.
- **P2.1-D02 — Desktop awareness:** Default metadata awareness, on-demand screen perception and optional continuous screen perception are supported as distinct levels. Continuous perception requires a visible indicator, configurable scope and immediate pause control. See ADR-013.
- **P2.1-D03 — Task queue:** A persistent, user-visible task queue with Queued, Running, Blocked, Paused, Completed, Cancelled and Failed states is part of the product direction. Objectives, plans and individual actions are represented separately.
- **P2.1-D09 — Busy-user behavior:** Suitable background-safe work may continue while the user is busy. Desktop-interactive work should avoid disrupting active user activity. Tasks should be paused, checkpointed or deferred when safe execution conditions are not satisfied. Exact thresholds and interruption rules remain for P2.2 and P2.4.

## 5.2 Remaining decisions

| ID | Question | Why it matters | Status |
|---|---|---|---|
| P2.1-D04 | Which window operations belong in the first vertical slice? The full product scope includes discovery, focus, move, resize, minimize, maximize, restore, close, launch and window arrangement. | Selects a small but convincing first implementation. | Open |
| P2.1-D05 | Should file-system operations be included in the first vertical slice, or should the first OS action remain limited to window interaction? | Controls risk and scope of the first working loop. | Open |
| P2.1-D06 | Which memory categories or data classes require explicit user approval before retention, and which may be autonomously retained under standing memory preferences? | Autonomous memory formation is approved, but sensitivity, retention and consent boundaries remain to be defined in P2.5/P2.6. | Open |
| P2.1-D07 | Should the character itself be user-draggable, and under what conditions may LIA reposition herself? | Defines control of LIA's own desktop position and spatial behavior. | Open |
| P2.1-D08 | Which input and interaction capabilities belong in the first vertical slice? The full product uses a hybrid model. | Separates eventual capability from initial implementation scope. | Open |
| P2.1-D10 | Which application categories should receive early integration support? | Prevents the first release from implicitly promising support for every application. | Open |

# 6. Candidate first vertical slice

This is a proposed integration target, not a finalized release
commitment.

The first vertical slice should prove one complete interaction loop
across the major subsystems without attempting the entire product:

1.  LIA appears as a desktop character and performs idle animation.
2.  The user initiates a voice conversation.
3.  LIA listens, responds, and speaks.
4.  The character's expressions and body language react to the
    interaction.
5.  LIA obtains basic desktop context, such as the active window,
    through the platform adapter.
6.  The user asks LIA to perform one supported window operation.
7.  LIA checks authorization, performs the operation, verifies the
    result where possible, and coordinates the character's movement or
    expression with the operation.
8.  LIA retains one appropriate conversational or task detail across a
    restart.

The exact operation and memory behavior remain subject to P2.1 review
and later acceptance criteria.

------------------------------------------------------------------------

# 7. Requirements traceability

This section will map each requirement to its source product capability,
architecture component, implementation work item, and test case as later
phases are completed.

  -----------------------------------------------------------------------
  Requirement range Product domain    Architecture      Test mapping
                                      mapping           
  ----------------- ----------------- ----------------- -----------------
  FR-001--009       Presence and      TBD in P3         TBD in P2.7
                    lifecycle                           

  FR-010--019       Environmental     TBD in P3         TBD in P2.7
                    awareness                           

  FR-020--029       Communication     TBD in P3         TBD in P2.7

  FR-030--039       Embodiment        TBD in P3         TBD in P2.7

  FR-040--049       OS interaction    TBD in P3         TBD in P2.7

  FR-050--059       Application and   TBD in P3         TBD in P2.7
                    file interaction                    

  FR-060--069       Memory            TBD in P3         TBD in P2.7

  FR-070--089       Autonomy and      TBD in P3         TBD in P2.7
                    tasks                               

  FR-090--099       Configuration and TBD in P3         TBD in P2.7
                    control                             

  FR-100--109       Runtime           TBD in P3         TBD in P2.7
                    self-management                     
  -----------------------------------------------------------------------

------------------------------------------------------------------------

### Supplemental traceability for expanded P2.1 requirements

| Requirement range | Product domain | Architecture / requirements follow-up |
|---|---|---|
| FR-110–119 | Spatial desktop awareness | P2.3 perception; P3 spatial representation and platform adapters |
| FR-120–129 | Expanded window management | P2.6 authorization; P3 window-management adapter |
| FR-130–139 | Application interaction and permissions | P2.6 per-application permission model; P3 application adapters |
| FR-140–149 | File-system interaction | P2.6 scoped file permissions; P3 file-system and file-manager interfaces |
| FR-150–159 | Desktop environment configuration | P2.3 context; P2.6 permissions; P3 platform adapters |
| FR-160–169 | Hybrid input interaction | P2.4 interaction; P2.6 authorization; P3 interaction abstraction |
| FR-170–189 | Task management and scheduling | P2.2 performance/resources; P2.6 authority; P3 scheduler and task runtime |
| FR-190–209 | Memory formation, retrieval and context assembly | P2.5 memory specification; P2.6 privacy/governance; P2.7 evaluation |

# 8. Decision register

  ----------------------------------------------------------------------------
  ID                Decision           Status              Rationale /
                                                           consequence
  ----------------- ------------------ ------------------- -------------------
  ADR-001           LIA is an embodied Approved            Establishes product
                    autonomous digital                     identity and
                    companion and                          evaluation
                    OS-native agent.                       baseline.

  ADR-002           Companionship and  Approved            Neither side is
                    OS assistance are                      subordinate to the
                    equally important                      other.
                    product goals.                         

  ADR-003           Use a layered      Approved at         Avoids collapsing
                    personality,       conceptual level    character identity
                    emotional-state,                       into a single
                    behavior,                              prompt.
                    expression, and                        
                    experience model.                      

  ADR-004           Separate           Approved            Preserves agency
                    independent                            while enforcing
                    intention from                         permission
                    execution                              boundaries.
                    authority.                             

  ADR-005           Treat social,      Approved            Allows independent
                    cognitive,                             configuration and
                    operational, and                       safeguards.
                    temporal autonomy                      
                    as separate                            
                    dimensions.                            

  ADR-006           Linux and Windows  Approved            Defines initial
                    are first-class                        platform scope.
                    initial platforms;                     
                    macOS is a future                      
                    extension.                             

  ADR-007           Use a shared core  Approved            Keeps
                    with separate OS                       platform-specific
                    adapters.                              behavior out of
                                                           shared agent logic.

  ADR-008           Platform adapters  Approved            Prevents false
                    must report                            cross-platform
                    capability                             parity assumptions.
                    limitations                            
                    honestly.                              

  ADR-009           Camera is disabled Approved at         Detailed permission
                    by default and     product-principle   flow remains in
                    requires explicit  level               P2.6.
                    activation.                            

  ADR-010           Screen and         Approved at         Detailed permission
                    microphone access  product-principle   and indicator
                    must be            level               requirements remain
                    user-controlled                        in P2.6.
                    and visible.                           

  ADR-011           P2 requirements    Approved            Reduces premature
                    should be                              architectural
                    established before                     lock-in.
                    committing to the                      
                    final technology                       
                    stack.                                 
  ----------------------------------------------------------------------------

### ADR-012 — Four configurable interaction modes

**Status:** Approved

LIA shall support push-to-talk, continuous conversation, wake-word activation, and text interaction. The modes shall be independently configurable. This provides multiple interaction styles while retaining user control over microphone access. Microphone access is not implied to be continuously active; it remains subject to explicit permission and a visible listening state.

### ADR-013 — Three-level desktop awareness

**Status:** Approved

Desktop awareness is divided into default metadata awareness, permissioned on-demand screen perception, and optional continuous screen perception. Continuous perception must include a visible indicator, configurable scope, and immediate pause control. Screen-content access is not implied by default metadata access.

------------------------------------------------------------------------

------------------------------------------------------------------------


### ADR-014 — Desktop as an interactive spatial environment

**Status:** Approved design direction

LIA treats the desktop as an environment containing identifiable, spatially related and manipulable objects. Product scope includes window management, application interaction, file-system interaction, desktop-environment interaction/configuration and keyboard/pointer input. These capabilities are subject to platform support and user permissions.

### ADR-015 — Spatial desktop awareness

**Status:** Approved design direction

LIA maintains an understanding of monitor and workspace topology, window geometry, object relationships and relevant environmental changes. Spatial information must distinguish direct observations from inferred relationships and remain synchronized with the changing desktop.

### ADR-016 — Per-application capability permissions

**Status:** Approved design direction

Application permissions distinguish Observe, Interact and Modify capabilities and are configurable per application. These permissions are independent. Modification permission does not automatically authorize every consequential operation; detailed scopes and additional authorization rules are defined in P2.6.

### ADR-017 — Direct and interactive file-system operation

**Status:** Approved design direction

LIA supports both direct file-system operations and embodied interaction through a file-manager interface. Both methods enforce the same file-system authorization boundaries and must provide verifiable outcomes.

### ADR-018 — Desktop-environment configuration

**Status:** Approved design direction

LIA's eventual capability scope includes navigating and configuring supported monitors, workspaces, panels, docks, launchers, desktop objects and display settings, subject to platform capabilities and authorization.

### ADR-019 — Hybrid computer interaction

**Status:** Approved design direction

LIA combines direct input simulation, accessibility or application interfaces, and visual perception-driven interaction. She selects an appropriate mechanism for each operation, while all mechanisms share common planning, authorization, execution-result and verification rules. This does not select specific implementation technologies.

### ADR-020 — Task hierarchy and persistent task queue

**Status:** Approved design direction

LIA maintains a user-visible persistent task queue and represents work as Objective → Plan → Actions. Task states include Queued, Running, Blocked, Paused, Completed, Cancelled and Failed. A task record includes origin, plan, dependencies, required capabilities/authorization basis, status, progress, execution history, blockers and outcome.

### ADR-021 — Separate task creation, planning and execution authority

**Status:** Approved design direction

Task creation, planning and execution are separate capabilities. LIA may identify and create useful tasks using relevant user memory, goals, conversations and permitted context. She may execute within valid standing authorization without seeking approval for every step. She notifies the user before autonomously initiated work; notification is not an approval request. Operations outside the authorization scope remain blocked pending authorization or plan revision.

### ADR-022 — Adaptive scheduling and non-interference

**Status:** Approved design direction

LIA schedules work using priority, urgency, deadlines, dependencies, resource needs and execution context. She supports immediate, background, idle-time, scheduled, deadline-driven, event-triggered and deferred execution. She may adapt schedules while respecting explicit user priorities and communicating meaningful changes. Suitable background work may continue while the user is busy; desktop-interactive work must avoid disrupting active user activity. Tasks should checkpoint and resume safely.

### ADR-023 — Hybrid, multi-layered memory

**Status:** Approved conceptual architecture

LIA's memory combines a searchable historical experience archive with organized long-term representations, including episodic, semantic, procedural, working and task memory. A temporal knowledge graph provides the relational backbone, while original source evidence remains accessible. No single memory representation is considered sufficient.

### ADR-024 — Autonomous memory formation and consolidation

**Status:** Approved conceptual architecture

LIA autonomously identifies candidate memories from permitted experiences and consolidates recurring or significant information. Explicit statements may establish high-confidence memories immediately; weaker inferences require stronger or repeated evidence. Original evidence must remain traceable, and derived summaries must not replace the historical record.

### ADR-025 — Associative memory reinforcement

**Status:** Approved conceptual design direction

Meaningful relationships between memories and entities may strengthen through repeated, sufficiently independent experiences and strong explicit evidence. Association strength, factual confidence, importance and retrieval activation remain separate. Mere co-occurrence or repeated retrieval is not sufficient evidence for reinforcement. Thresholds and maintenance parameters require empirical calibration.

### ADR-026 — Memory history, provenance and temporal validity

**Status:** Approved conceptual design direction

Memory records retain source provenance and distinguish the time an event was valid from the time LIA learned or recorded it. Contradictions and superseded information are reconciled without erasing historical evidence. Retention, sensitive-data handling, correction and deletion policies remain detailed P2.5/P2.6 work.

### ADR-027 — Context-aware hybrid memory retrieval

**Status:** Approved conceptual architecture

LIA's retrieval engine determines the information need, plans retrieval across semantic, lexical, structured, graph, temporal and source-archive representations, expands relevant graph relationships in a bounded manner, validates evidence and assembles a bounded working context with source references and uncertainty labels.

### ADR-028 — Memory retrieval traceability and evaluation

**Status:** Approved design direction

Memory retrieval should expose why information was selected and retain enough trace information to debug extraction, entity resolution, graph traversal, ranking, temporal resolution and context assembly. Evaluation must include exact recall, multi-session recall, multi-hop reasoning, temporal reasoning, contradiction handling, provenance, abstention, correction, deletion and task resumption.

# 9. Change log

  ------------------------------------------------------------------------
  Version                 Date                    Changes
  ----------------------- ----------------------- ------------------------
  1.0.0                   12 Nov 2026             Consolidated the
                                                  approved Phase 1 product
                                                  baseline, roadmap,
                                                  decision register, and
                                                  initial P2.1
                                                  functional-requirement
                                                  draft.

  1.0.1                   12 Nov 2026             Approved all four voice
                                                  interaction modes;
                                                  updated FR-020–FR-025
                                                  and FR-028; recorded
                                                  ADR-012 and closed
                                                  P2.1-D01.

  1.0.2                   12 Nov 2026             Approved the three-level
                                                  desktop awareness model;
                                                  revised FR-010–FR-019;
                                                  recorded ADR-013 and
                                                  closed P2.1-D02.

  1.1.0                   13 Nov 2026             Consolidated P2.1
                                                  discussions through
                                                  task scheduling, spatial
                                                  desktop interaction,
                                                  hybrid input, application
                                                  and file permissions,
                                                  desktop configuration,
                                                  and the conceptual
                                                  memory architecture,
                                                  associative reinforcement
                                                  and retrieval design.

  ------------------------------------------------------------------------

------------------------------------------------------------------------

# 10. Current working state

**Current phase:** P2 --- Requirements Engineering\
**Active workstream:** P2.1 --- Functional Requirements\
**Current status:** P2.1 design baseline expanded; detailed functional requirements remain under review\
**Next action:** Continue P2.1 with memory governance and integrity, then review embodiment and runtime self-management. Resolve remaining first-release questions and formalize acceptance criteria in P2.7. Do not move to technology selection yet.

## Phase 2 exit criteria

Phase 2 is complete only when:

-   Functional requirements are reviewed, uniquely identified, and
    approved.
-   Non-functional requirements have measurable targets or explicit TBD
    owners.
-   Context and perception boundaries are defined.
-   Interaction requirements cover voice, text, interruption, presence,
    and embodiment.
-   Memory requirements define persistence, retrieval, correction, and
    deletion behavior.
-   Autonomy, security, and permission requirements define execution
    authority and user controls.
-   Acceptance criteria cover all P0 requirements and the agreed first
    vertical slice.
-   Requirements are internally consistent and traceable into the
    architecture phase.

## Working rule

When a decision changes, update the relevant section, decision register,
and change log. Keep requirement IDs stable; do not recycle IDs when
requirements are removed. Mark obsolete requirements as rejected or
superseded and record the replacement ID.
