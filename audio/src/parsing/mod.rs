use serde::{Deserialize, Serialize};
use std::process::Command;
use reqwest::Client;
use std::fs;
use crate::error::AppError;




// main splitter so that the LLM can handle the text that is coming in
fn split_into_chunks(input: &str, chunk_size: usize) -> Vec<String> {
    println!("Splitting the text appropriately...");
    let chunks: Vec<String> = input
        .chars() // Convert the string into an iterator of characters
        .collect::<Vec<char>>() // Collect characters into a vector
        .chunks(chunk_size) // Split vector into chunks
        .map(|chunk| chunk.iter().collect()) // Convert each chunk back into a String
        .collect(); // Collect all chunks into a Vector

    println!("{:?}", chunks);
    return chunks;
}

// Topics parser to check how the llm handles topic parsing
fn parse_topics(response: &str) -> (Vec<String>, String) {
    let start_tag = "<topic>";
    let end_tag = "</topic>";
    let mut finished = Vec::new();
    let mut temp_buf = String::new();
    let mut flag = false;
    let mut i = 0;

    while i < response.len() {
        // Check for the start of a tag
        if response.as_bytes()[i] == b'<' {
            // Check if it's an end tag
            if i + 1 < response.len() && response.as_bytes()[i + 1] == b'/' {
                if response[i..].starts_with(end_tag) {
                    // If currently capturing, push to finished and reset
                    if flag {
                        finished.push(temp_buf.clone());
                        temp_buf.clear();
                        flag = false;
                    }
                    i += end_tag.len();
                    continue;
                }
            } else {
                // It's a start tag
                if response[i..].starts_with(start_tag) {
                    flag = true;
                    i += start_tag.len();
                    continue;
                }
            }
        }

        // If we are between tags, add to temp_buf
        if flag {
            temp_buf.push(response.as_bytes()[i] as char);
        }
        i += 1;
    }

    // Any data left after the last tag is considered unfinished
    let unfinished = temp_buf;

    (finished, unfinished)
}

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    role: String,
    content: String,
}


#[derive(Serialize, Deserialize, Debug)]
struct ApiResponse {
    model: String,
    created_at: String,
    message: Message,
    done: bool,
}

pub struct Sst {
    audio_file: String,
    model_path: String,
}

impl Sst {
    pub fn new(audio_file: String, model_path: String) -> Self {
        Self {
            audio_file,
            model_path,
        }
    }

    fn extract_text_from_audio(&self) -> Result<String, AppError> {
        println!("B4");
        let output_txt = format!("{}.txt", self.audio_file);
        println!("1");
        let command = format!("/Users/j-supha/FFMPEG/whisper.cpp/main --model {} --output-txt {} {}", self.model_path, output_txt, self.audio_file);
        println!("2");
        let output = Command::new("sh")
            .arg("-c")
            .arg(&command)
            .output()?;
        println!("3");
        if !output.stdout.is_empty() {
            println!("{}", String::from_utf8_lossy(&output.stdout));
        }
        println!("4");
        if !output.stderr.is_empty() {
            eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        }
        println!("5");
        // Read the output from the text file
        let text = fs::read_to_string(output_txt)?;
        println!("6 {:?}", text);
        Ok(text)
    }

    // Method to send extracted text to API and handle response
    async fn send_text_to_api(&mut self, text: String) -> Result<(), AppError> {
        println!("Sending text to API...");
        let client = Client::new();
        let my_vec = split_into_chunks(&text, 2000);
        let mut f = String::new();
        let mut total: Vec<String> = Vec::new();
        for vectors in my_vec {
            println!("Sending text to API for topic parsing");
            
            let insert = format!("{} {}", f,vectors);
            let wild = format!("NEW!: {} {}\n\n\n", f,vectors);
            fs::write("/Users/j-supha/Desktop/Personal_AI/FFMPEG/audio/whatIsHappenning.txt", wild).expect("unable to write file");
            let request_body = serde_json::json!({
                "model": "llama3",
                "messages": [
                    {
                        "role": "system",
                        "content": "Your task is to group the following conversation between multiple parties by sub topic. You will be given a chunk of text and you have to group the text into a segments where each segment relates to a specific topic of conversation. I will give you a piece of text and you will insert <topic> tag at the beginning of the segment and the </topic> tag at the end of the segment. You need to return the entire segment with the tags in place. If you determine that the segment has not finished then do not add the final </topic> tag. Make sure that all the given text is encapsulated within a set of <topic> </topic> tags. You may encouter a case where the user has written a <topic> already. Just determine when that topic ends and then continue as normal"
                    },
                    {
                        "role": "user",
                        "content": "-Hey, thank you for having me, I appreciate that.
                        -You're the only guy I've ever had in the studio where when I showed up, you were workingout.
                        -That's what I do, man, that's my life. That's my life.
                        -It's pretty crazy though. I mean how much time did you have when you got here?
                        -I got here about an hour early.
                        -Oh okay. Yeah. Okay.
                        -We got a little early.
                        -So I got here shirt off doing chin ups. It was hilarious. I didn't get my camera out in timebefore you saw me.
                        -I wanted to take some pictures.
                        -Well maybe next time.
                        -Next time. Well I'll catch you after the show. 
                        You are a guy that for a lot of people you sort of embody the idea of hardening your mind and figuring out a way To do things that most people think are impossible That's you've sort of become that guy over your life and you become that guy for a lot of people including me online We've talked about you on the podcast a ton of times.
                        So having you in here has been very exciting to me.
                        -I appreciate that. Thank you.
                        -How'd you become that guy?
                        -You know what, I grew up not that guy.
                        -Yeah.
                        -So a lot of people put a title on me. They want to, they see me now. They see me now asthe guy that with his shirt off, who can do 4, 030 pull-ups in 17 hours, who can run 205miles in 39 hours, who can do all this crazy shit.
                        -But what they don't understand is they don't understand the journey that it took me to getto this point and What got me to this point was I was just the opposite of what I am today Iwas that guy who ran away from Absolutely everything that I got in front of me, but Notmany people knew that I had 2 people had the like the real me was like this very scaredinsecure Stuttering got beat up by his dad all this kind of stuff And I built this fake personthat walked around like my shit didn't stink you know you know yeah so that was that'skind of how I did it and I do process of time I realized that I was lying to myself and lyingto people
                        -but that it's a fascinating journey though because you are that guy now right Yougenuinely are legit badass right at 1 point in time you were a legit terrified person
                        -Yes,
                        -so what was the process like how did how did you step forth?
                        -Well, it's a it's a long process,
                        -right?
                        -My dad beat the shit out me was growing up We I was the first black baby born in thishospital called Miller Fillmore in Buffalo, New York My dad owns skating rinks. He ownedbars. He ran prostitutes from Canada to Buffalo, New York My dad was a big-time pimpbig-time anything bad about a person big-time hustler He was American you know that I'mwith them Daniel Washington.
                        -He was that but not that bad
                        -right You
                        -know he wasn't that big, but that's what it reminds me of he was that kind of guy and beatthe shit out of me, the shit out of my mom. There was an incident 1 time when my momgot knocked out on top of the stairs and he drug her down the stairs by her hair. And at 6years old, I'll never forget this.
                        -In my mind, I was always afraid. My whole life I was afraid, but I had this fucking voice,this conscience, that would always be battling me, saying, hey, you gotta get up and dosomething. I didn't wanna do shit. You know, I was just afraid, but that voice would forceme to get up, and my dad, I'd try to beat him up, whatever, at 6, and I'd get my ass kicked.
                        -So this went on for several years and I have a big time learning disability. My dad didn'tbelieve in us going to school. So my dad it was about the business, the skating rink andthe bar. So the skating rink opened about 07:00 at night, and this is when I was able towalk so about 5 you know 4 or 5 6 years old 8 9 and I go to this you know skating rink07:00 at night And I worked the skating rink until 10 at night And then We would scrapethe gum off the floors and we cleaned the whole skating rink up.
                        -And then my dad had an office. And my brother and myself would sleep in the office. Mymom would go upstairs and work the bar until 03:00 in the morning. And then theycleaned the bar up.
                        -So after all that shit was done with, going to school rarely happened. So when I went toschool, I was all kind of you know, my my learning disability. I had social anxiety I was justa jacked-up kid from living in this tortured home from the outside looking in we live in theall-white neighborhood and then we would travel to the ghetto of Buffalo, New York wherethe skating rink was at. So we you know we worked around mostly blacks and I livedaround mostly whites but no 1 knew what was going on that house that on 201 ParadiseRoad.
                        -You know it's crazy but my mom got courage to finally leave him when I was about 8 yearsold we moved to a small town in Brazil Indiana and That's when the real war started for meand Brazil Indiana is a small town great people a lot of great people and I say that becausea lot of people get offended and I'm gonna get to the point why they get offended. Therewas about maybe 10 black families at about 10, 000 people in the town and in 1995 theKKK marched in the 4th of July parade. So this was a, not everybody was racist. There's alot of good people some the best people I knew was there But there's also a lot of racismthere so me being 1 of the few black kids in that you know in the area You know it kind ofhaunts you I had stuff on my notebook.
                        -You know nigga We're gonna kill you on my Spanish notebook. They had that on my carnigga we're gonna kill you this is early 90s and So even though I showed it didn't hurt meit was jacking me up So all the insecurities I have when I was a kid with my father I'vemoved into this area here And it just got worse and worse and worse. And this shithaunted me. And that voice that I talked about, it kept talking louder and louder andlouder, but I was doing nothing about it.
                        -And I decided to make moves. And I cheated all through school. And it's kind of humblingto talk about my story sometimes and it's also embarrassing but it's real. It's who the fuckI am."
                    },
                    {
                        "role": "assistant",
                        "content": "<topic>
                        -Hey, thank you for having me, I appreciate that.
                        -You're the only guy I've ever had in the studio where when I showed up, you were workingout.
                        -That's what I do, man, that's my life. That's my life.
                        -It's pretty crazy though. I mean how much time did you have when you got here?
                        -I got here about an hour early.
                        -Oh okay. Yeah. Okay.
                        -We got a little early.
                        -So I got here shirt off doing chin ups. It was hilarious. I didn't get my camera out in timebefore you saw me.
                        -I wanted to take some pictures.
                        -Well maybe next time.
                        -Next time. Well I'll catch you after the show.</topic> 
                        <topic>
                        You are a guy that for a lot of people you sort of embody the idea of hardening your mind and figuring out a way To do things that most people think are impossible That's you've sort of become that guy over your life and you become that guy for a lot of people including me online We've talked about you on the podcast a ton of times.
                        So having you in here has been very exciting to me.
                        -I appreciate that. Thank you.
                        -How'd you become that guy?
                        -You know what, I grew up not that guy.
                        -Yeah.
                        -So a lot of people put a title on me. They want to, they see me now. They see me now asthe guy that with his shirt off, who can do 4, 030 pull-ups in 17 hours, who can run 205miles in 39 hours, who can do all this crazy shit.
                        -But what they don't understand is they don't understand the journey that it took me to getto this point and What got me to this point was I was just the opposite of what I am today Iwas that guy who ran away from Absolutely everything that I got in front of me, but Notmany people knew that I had 2 people had the like the real me was like this very scaredinsecure Stuttering got beat up by his dad all this kind of stuff And I built this fake personthat walked around like my shit didn't stink you know you know yeah so that was that'skind of how I did it and I do process of time I realized that I was lying to myself and lyingto people
                        -but that it's a fascinating journey though because you are that guy now right Yougenuinely are legit badass right at 1 point in time you were a legit terrified person
                        -Yes,
                        </topic>
                        <topic>
                        -so what was the process like how did how did you step forth?
                        -Well, it's a it's a long process,
                        -right?
                        -My dad beat the shit out me was growing up We I was the first black baby born in thishospital called Miller Fillmore in Buffalo, New York My dad owns skating rinks. He ownedbars. He ran prostitutes from Canada to Buffalo, New York My dad was a big-time pimpbig-time anything bad about a person big-time hustler He was American you know that I'mwith them Daniel Washington.
                        -He was that but not that bad
                        -right You
                        -know he wasn't that big, but that's what it reminds me of he was that kind of guy and beatthe shit out of me, the shit out of my mom. There was an incident 1 time when my momgot knocked out on top of the stairs and he drug her down the stairs by her hair. And at 6years old, I'll never forget this.
                        -In my mind, I was always afraid. My whole life I was afraid, but I had this fucking voice,this conscience, that would always be battling me, saying, hey, you gotta get up and dosomething. I didn't wanna do shit. You know, I was just afraid, but that voice would forceme to get up, and my dad, I'd try to beat him up, whatever, at 6, and I'd get my ass kicked.
                        -So this went on for several years and I have a big time learning disability. My dad didn'tbelieve in us going to school. So my dad it was about the business, the skating rink andthe bar. So the skating rink opened about 07:00 at night, and this is when I was able towalk so about 5 you know 4 or 5 6 years old 8 9 and I go to this you know skating rink07:00 at night And I worked the skating rink until 10 at night And then We would scrapethe gum off the floors and we cleaned the whole skating rink up.
                        -And then my dad had an office. And my brother and myself would sleep in the office. Mymom would go upstairs and work the bar until 03:00 in the morning. And then theycleaned the bar up.
                        -So after all that shit was done with, going to school rarely happened. So when I went toschool, I was all kind of you know, my my learning disability. I had social anxiety I was justa jacked-up kid from living in this tortured home from the outside looking in we live in theall-white neighborhood and then we would travel to the ghetto of Buffalo, New York wherethe skating rink was at. So we you know we worked around mostly blacks and I livedaround mostly whites but no 1 knew what was going on that house that on 201 ParadiseRoad.
                        -You know it's crazy but my mom got courage to finally leave him when I was about 8 yearsold we moved to a small town in Brazil Indiana and That's when the real war started for meand Brazil Indiana is a small town great people a lot of great people and I say that becausea lot of people get offended and I'm gonna get to the point why they get offended. Therewas about maybe 10 black families at about 10, 000 people in the town and in 1995 theKKK marched in the 4th of July parade. So this was a, not everybody was racist. There's alot of good people some the best people I knew was there But there's also a lot of racismthere so me being 1 of the few black kids in that you know in the area You know it kind ofhaunts you I had stuff on my notebook.
                        -You know nigga We're gonna kill you on my Spanish notebook. They had that on my carnigga we're gonna kill you this is early 90s and So even though I showed it didn't hurt meit was jacking me up So all the insecurities I have when I was a kid with my father I'vemoved into this area here And it just got worse and worse and worse. And this shithaunted me. And that voice that I talked about, it kept talking louder and louder andlouder, but I was doing nothing about it.
                        -And I decided to make moves. And I cheated all through school. And it's kind of humblingto talk about my story sometimes and it's also embarrassing but it's real. It's who the fuckI am.
                        </topic>
                        "
                     },
                     {
                        "role": "user",
                        "content": "Okay, everybody's here, that's awesome.
                        We're gonna upload and share a doc
                        into the news research drive and share with all you guys.
                        It is a Forge project management doc
                        that's basically an analog to the network doc
                        that was made put together
                        from some of the compiled notes that we've all shared.
                        Just wanna go over it, talk about comms,
                        talk about responsibilities, talk about the project scope,
                        et cetera, and wanna run through all of it,
                        you guys, before the stand up.
                        Ari, I'm happy to say you have no responsibilities
                        inside of the current doc as it exists today.
                        You have plenty of work cut out for you
                        on the world segment already.
                        And before I just jump into Forge,
                        I'll just say on the world soon, et cetera.
                        Thanks to the very hard work of VORPs
                        and we're still cleaning up now by the new props,
                        able to get really, really good results using Stonnet
                        and even haiku over opus,
                        like better than opus as results.
                        So we're getting ready to bring that to you, Ari,
                        next week to actually integrate into
                        world, sim, and world clients.
                        - Sick, I'm very excited.
                        - Yeah, we shall save the monies.
                        And we believe we will finally be a leg up over a website
                        in terms of actual site functionality design,
                        but we'll let everyone be the judge of that
                        when we test during the next needs on Tuesday.
                        And that being said, I'm just uploading the document
                        to drive, I'm gonna share with everyone
                        and I can stream it too.
                        And tag, Kannen, thank you guys for coming
                        very much, I'm happy you guys are here.
                        I'm mostly being looking for feedback
                        and thoughts from Shannon and Jay today
                        right now on this before opening it up to everybody
                        during the standup, if that's all right.
                        Just share.
                        - I think Kannen's just here 'cause he won't stay updated
                        so that he can interact with the community better.
                        - Perfect, that makes total sense.
                        We do, I do wanna figure out about community management
                        plans and community leading efforts, Kannen.
                        So that's something we will talk about.
                        I just wanna get the deep brass tags down first.
                        I'm gonna start the show now.
                        - Is that shaded in the channel?
                        I can't see anything.
                        - No, I'm sending it to everybody's news emails right now.
                        - That's everyone.
                        Okay, I think that's everyone that needs to see it right now.
                        , I'll start streaming now also.
                        Yay.
                        (mumbling)
                        (mumbling)
                        (mumbling)
                        Great.
                        (mumbling)
                        (mumbling)
                        (mumbling)
                        Okay, can everybody see okay?
                        (mumbling)
                        - Yep.
                        Oh yeah, there we go.
                        - Hey Greg.
                        - I'm not on the best internet,
                        so it might not be super clear
                        which is why I wanna share it to everybody.
                        I'm on the Kannen.
                        You know, what's your email?
                        - Kannen.newsresearch.com.
                        - Okay, you guys should all have access now
                        in case things are not working out.
                        So I based this off of the conciliency PM doc.
                        We have a couple pieces of this that are totally complete
                        and then pieces of this that I wanna add to.
                        And of course, like we should be iteratively working on all of it.
                        But we've got an overview of what's in scope
                        and what's out of scope very importantly as well.
                        So we can lock in exactly what we need to be focused on.
                        Then just a general matrix for like,
                        who's responsible for what, who's accountable for doing it,
                        who's gonna be helping with it.
                        So people know who to reach out to
                        when they have certain questions
                        about different pieces of the project.
                        I don't want five people messaging one person
                        about each thing if that person's not the person
                        who is the point man for that project.
                        So we put that together too.
                        Basic contact sheet, I'll have you guys
                        just fill out your own info there.
                        Project watchlist, I don't have like four projects there now
                        but we can go through way more,
                        way more agent builders that we've seen already.
                        Open issues, I haven't worked on this yet.
                        I wanted to do this together.
                        Back end design right now just pointing
                        towards Shannon's forge requirement stock.
                        But of course that's going to be heavily restructured
                        given the movement to cloud and the decision
                        to kind of work on a chat interface
                        based initial interactions.
                        The questionnaire, we have some
                        of the questionnaire ready so far
                        but we want to continue working on it.
                        Launch plan, it's work in progress.
                        We have some of it so far.
                        And then budget day will work on that.
                        That's the last piece.
                        So, just kind of going over the scope.
                        Oops, I pulled out the budget objective, sorry.
                        So I know we've had a million different discussions
                        about forge and all the things that's important for it
                        but I think the most important thing about forge
                        is the initial reason that we raised
                        which is like tracking robust agents,
                        making sure that people can actually use something
                        that works and it works without any hassle on their end.
                        They don't have to keep checking in on it.
                        Any errors, whatever, it's handled.
                        That's the most important thing that we can solve
                        and I want everything else to kind of be seen
                        as a second order problem after this mentality
                        of we want seamless automation of LMs.
                        Now when it comes to objectives like hopefully
                        this is stuff that as the forge team we've all gone over
                        but I just wanted to organize it all for us.
                        Today we need to figure out the perfect reasoning
                        and scripting stack and language that we're going to use
                        in order to actually solve that issue.
                        I know we've come quite far but I also know
                        that Shannon is going full time trying to do this
                        and once Vogel comes on board,
                        hopefully that'll catalyze that work.
                        Secondly comes down to the actual kind of like consumer side.
                        Like we need an interface and a product experience
                        that lets people utilize this without feeling overwhelmed,
                        without feeling like bored or like I want to click out of this,
                        I don't really want to work on this, et cetera.
                        One second.
                        Yeah.
                        And then finally the, oh, I should write anything for coming.
                        And then so we're just going over the PM doc.
                        I just want people to kind of share their thoughts
                        and pieces as the internal forge team
                        before I went over it with everybody in the stand up.
                        And then of course, following us being able to say,
                        we have solved this reasoning stack for some initial cases
                        and we've created an experience that will funnel
                        for people very easily and seamlessly.
                        That's when we can talk about kind of the Xcode
                        and the like here's a toolkit for devs to build on a piece
                        but I really want to look at dev buildability
                        and customization for devs to be a secondary
                        like second order issue and not something
                        that we should primarily concern ourselves with.
                        I really want it to be seen in this like one, two, three
                        sort of hierarchy of what we need to do.
                        So then I follow this template of what do we research?
                        What are we designing?
                        What are we developing?
                        What are we launching?
                        How are we drawing?
                        So the initial pieces that we need to research
                        are these fundamental use cases
                        and we can put together the fundamental nodes
                        that are common across those different use cases
                        for end users specifically for the consumer
                        that we want to target specifically.
                        And in order to figure that out,
                        we need to make sure that questionnaire is robust.
                        We need to put it out and get more information from people
                        and what they want to automate in their day-to-day life.
                        To like the research question, massive open question
                        of the scripting system that Shannon is building right now,
                        how do we simultaneously make sure
                        that it's something that can accommodate various nodes
                        without feeling bloated
                        and still having the same seamless experience for a user
                        and still having it feel like it's the same interaction
                        as if there was just one or two nodes available.
                        And thirdly, in the situation that we are releasing
                        a toolkit, we want to be able to find out
                        how do we actually integrate the automations
                        that are created by developers
                        into the final product experience?
                        So I think these are the three big pieces of research
                        so you can simply break down to consumer interest
                        on use case, actual flexibility of a complex system,
                        and how to bring back developers' ideas
                        or automations into that one centralized system.
                        Then in terms of design,
                        the initial UI is extremely important.
                        Right now, what we've been leaning on more than anything
                        is building on top of an adaptive chat interface,
                        what we've found from a lot of mainstream outlets,
                        their main concern is bots,
                        and the way that they look at bots
                        and the way they look at AI,
                        there's no real distinction between Claude and GPT
                        putting hundreds of millions of dollars towards AI
                        and somebody like Perplexity putting
                        three agent use cases on top of it,
                        like Search or Play with Docs.
                        So I want to lean in on the Perplexity side of this,
                        considering we have the ability to use open models
                        and we have the ability to do a lot more customizations
                        than they do, still being able to serve it in a simple way
                        and have people be like, wow,
                        this is far more useful than GPT or something else I'm using,
                        allows us to be much more of a direct competitor.
                        So I'm very interested in this being the design issue
                        that we want to tackle.
                        Sorry.
                        Next, it's important for us to design
                        the kind of intuitive consumer funnels.
                        This is like more of a retention
                        and growth research kind of thing,
                        to figure out, click to click button to button,
                        what is gonna keep users on?
                        What is gonna keep users saying,
                        this is not boring.
                        This is something that I feel like
                        is just one seamless flow of action.
                        And not, oh my God, I have to learn 50 things.
                        There's 100 buttons here, I don't know what I'm doing.
                        We want simple funnels.
                        Another thing that we need to design of course is the,
                        as we said here, the reasoning and scripting system
                        for this chat interface.
                        The back end nodes and architecture links directly to the,
                        sorry, I gotta fix that link.
                        Should be linking to this.
                        Which is the previous node-based technical overview
                        of what's up in all the nodes that Shannon had built out.
                        And this is something that we want to of course,
                        adapt towards the cloud and the chat interface,
                        but it contains all the technical information
                        of what we already have available.
                        So it's very important that we continue to design
                        on that end and fix that link.
                        And all you guys, you should have this shared with you now.
                        So you should be able to look alongside with me,
                        feel free to click through.
                        And then finally, we need to design
                        like the marketing and narrative.
                        We're working with Riva and working with Mike on this
                        on making sure that this is something that we can ship
                        as a story and we can explain to people
                        the importance of this seamlessness
                        and show them how useful something like this
                        can be malleting by example.
                        A development comes down to this too.
                        For development, we want to get
                        those fundamental common nodes down.
                        The cloud is really useful.
                        Shannon, I'd love if you want to expand on this
                        because we can implement any of the ML libraries we want.
                        We can build just one build for ourselves.
                        We don't have to try to accommodate
                        for every single type of system
                        like we did with the old forge.
                        - Yeah. - Yeah.
                        - Yeah, I was gonna say like, I know that like,
                        this is something that we were discussing
                        sort of in terms of fort planning.
                        You can see everyone else that wasn't there.
                        But like how we were building forge
                        is it was the tool we personally would want.
                        But where all people would, we're like, you know,
                        stacks of like, you know, multiple 40 90s
                        and $10,000 MacBooks and stuff.
                        The reality is the average consumer
                        wouldn't even be able to run like forge.
                        And even if it was like an agent with like two or three nodes
                        and it was just like homey 70
                        and would still just be so slow
                        and painful on a bad user experience.
                        So the advantage of moving the cloud is that, you know,
                        like we can, we can manage all that.",
                     },
                     {
                        "role": "assistant",
                        "content": "
                                    <topic>- Okay, everybody's here, that's awesome.
                                    We're gonna upload and share a doc
                                    into the news research drive and share with all you guys.
                                    It is a Forge project management doc
                                    that's basically an analog to the network doc
                                    that was made put together
                                    from some of the compiled notes that we've all shared.
                                    Just wanna go over it, talk about comms,
                                    talk about responsibilities, talk about the project scope,
                                    et cetera, and wanna run through all of it,
                                    you guys, before the stand up.
                                    Ari, I'm happy to say you have no responsibilities
                                    inside of the current doc as it exists today.
                                    You have plenty of work cut out for you
                                    on the world segment already.
                                    And before I just jump into Forge,
                                    I'll just say on the world soon, et cetera.
                                    Thanks to the very hard work of VORPs
                                    and we're still cleaning up now by the new props,
                                    able to get really, really good results using Stonnet
                                    and even haiku over opus,
                                    like better than opus as results.
                                    So we're getting ready to bring that to you, Ari,
                                    next week to actually integrate into
                                    world, sim, and world clients.
                                    - Sick, I'm very excited.
                                    - Yeah, we shall save the monies.
                                    And we believe we will finally be a leg up over a website
                                    in terms of actual site functionality design,
                                    but we'll let everyone be the judge of that
                                    when we test during the next needs on Tuesday.
                                    And that being said, I'm just uploading the document
                                    to drive, I'm gonna share with everyone
                                    and I can stream it too.
                                    And tag, Kannen, thank you guys for coming
                                    very much, I'm happy you guys are here.
                                    I'm mostly being looking for feedback
                                    and thoughts from Shannon and Jay today
                                    right now on this before opening it up to everybody
                                    during the standup, if that's all right.
                                    Just share.
                                    - I think Kannen's just here 'cause he won't stay updated
                                    so that he can interact with the community better.
                                    - Perfect, that makes total sense.
                                    We do, I do wanna figure out about community management
                                    plans and community leading efforts, Kannen.
                                    So that's something we will talk about.
                                    I just wanna get the deep brass tags down first.
                                    I'm gonna start the show now.
                                    </topic>
                                    <topic>
                                    - Is that shaded in the channel?
                                    I can't see anything.
                                    - No, I'm sending it to everybody's news emails right now.
                                    - That's everyone.
                                    Okay, I think that's everyone that needs to see it right now.
                                    , I'll start streaming now also.
                                    Yay.
                                    (mumbling)
                                    (mumbling)
                                    (mumbling)
                                    Great.
                                    (mumbling)
                                    (mumbling)
                                    (mumbling)
                                    Okay, can everybody see okay?
                                    (mumbling)
                                    - Yep.
                                    Oh yeah, there we go.
                                    - Hey Greg.
                                    - I'm not on the best internet,
                                    so it might not be super clear
                                    which is why I wanna share it to everybody.
                                    I'm on the Kannen.
                                    You know, what's your email?
                                    - Kannen.newsresearch.com.
                                    - Okay, you guys should all have access now
                                    in case things are not working out.
                                    So I based this off of the conciliency PM doc.
                                    We have a couple pieces of this that are totally complete
                                    and then pieces of this that I wanna add to.
                                    And of course, like we should be iteratively working on all of it.
                                    But we've got an overview of what's in scope
                                    and what's out of scope very importantly as well.
                                    So we can lock in exactly what we need to be focused on.
                                    Then just a general matrix for like,
                                    who's responsible for what, who's accountable for doing it,
                                    who's gonna be helping with it.
                                    So people know who to reach out to
                                    when they have certain questions
                                    about different pieces of the project.
                                    I don't want five people messaging one person
                                    about each thing if that person's not the person
                                    who is the point man for that project.
                                    So we put that together too.
                                    Basic contact sheet, I'll have you guys
                                    just fill out your own info there.
                                    Project watchlist, I don't have like four projects there now
                                    but we can go through way more,
                                    way more agent builders that we've seen already.
                                    Open issues, I haven't worked on this yet.
                                    I wanted to do this together.
                                    Back end design right now just pointing
                                    towards Shannon's forge requirement stock.
                                    But of course that's going to be heavily restructured
                                    given the movement to cloud and the decision
                                    to kind of work on a chat interface
                                    based initial interactions.
                                    The questionnaire, we have some
                                    of the questionnaire ready so far
                                    but we want to continue working on it.
                                    Launch plan, it's work in progress.
                                    We have some of it so far.
                                    And then budget day will work on that.
                                    That's the last piece.
                                    So, just kind of going over the scope.
                                    Oops, I pulled out the budget objective, sorry.
                                    So I know we've had a million different discussions
                                    about forge and all the things that's important for it
                                    but I think the most important thing about forge
                                    is the initial reason that we raised
                                    which is like tracking robust agents,
                                    making sure that people can actually use something
                                    that works and it works without any hassle on their end.
                                    They don't have to keep checking in on it.
                                    Any errors, whatever, it's handled.
                                    That's the most important thing that we can solve
                                    and I want everything else to kind of be seen
                                    as a second order problem after this mentality
                                    of we want seamless automation of LMs.
                                    Now when it comes to objectives like hopefully
                                    this is stuff that as the forge team we've all gone over
                                    but I just wanted to organize it all for us.
                                    Today we need to figure out the perfect reasoning
                                    and scripting stack and language that we're going to use
                                    in order to actually solve that issue.
                                    I know we've come quite far but I also know
                                    that Shannon is going full time trying to do this
                                    and once Vogel comes on board,
                                    hopefully that'll catalyze that work.
                                    Secondly comes down to the actual kind of like consumer side.
                                    Like we need an interface and a product experience
                                    that lets people utilize this without feeling overwhelmed,
                                    without feeling like bored or like I want to click out of this,
                                    I don't really want to work on this, et cetera.
                                    One second.
                                    Yeah.
                                    And then finally the, oh, I should write anything for coming.
                                    And then so we're just going over the PM doc.
                                    I just want people to kind of share their thoughts
                                    and pieces as the internal forge team
                                    before I went over it with everybody in the stand up.
                                    And then of course, following us being able to say,
                                    we have solved this reasoning stack for some initial cases
                                    and we've created an experience that will funnel
                                    for people very easily and seamlessly.
                                    That's when we can talk about kind of the Xcode
                                    and the like here's a toolkit for devs to build on a piece
                                    but I really want to look at dev buildability
                                    and customization for devs to be a secondary
                                    like second order issue and not something
                                    that we should primarily concern ourselves with.
                                    I really want it to be seen in this like one, two, three
                                    sort of hierarchy of what we need to do.
                                    So then I follow this template of what do we research?
                                    What are we designing?
                                    What are we developing?
                                    What are we launching?
                                    How are we drawing?
                                    So the initial pieces that we need to research
                                    are these fundamental use cases
                                    and we can put together the fundamental nodes
                                    that are common across those different use cases
                                    for end users specifically for the consumer
                                    that we want to target specifically.
                                    And in order to figure that out,
                                    we need to make sure that questionnaire is robust.
                                    We need to put it out and get more information from people
                                    and what they want to automate in their day-to-day life.
                                    To like the research question, massive open question
                                    of the scripting system that Shannon is building right now,
                                    how do we simultaneously make sure
                                    that it's something that can accommodate various nodes
                                    without feeling bloated
                                    and still having the same seamless experience for a user
                                    and still having it feel like it's the same interaction
                                    as if there was just one or two nodes available.
                                    And thirdly, in the situation that we are releasing
                                    a toolkit, we want to be able to find out
                                    how do we actually integrate the automations
                                    that are created by developers
                                    into the final product experience?
                                    So I think these are the three big pieces of research
                                    so you can simply break down to consumer interest
                                    on use case, actual flexibility of a complex system,
                                    and how to bring back developers' ideas
                                    or automations into that one centralized system.
                                    Then in terms of design,
                                    the initial UI is extremely important.
                                    Right now, what we've been leaning on more than anything
                                    is building on top of an adaptive chat interface,
                                    what we've found from a lot of mainstream outlets,
                                    their main concern is bots,
                                    and the way that they look at bots
                                    and the way they look at AI,
                                    there's no real distinction between Claude and GPT
                                    putting hundreds of millions of dollars towards AI
                                    and somebody like Perplexity putting
                                    three agent use cases on top of it,
                                    like Search or Play with Docs.
                                    So I want to lean in on the Perplexity side of this,
                                    considering we have the ability to use open models
                                    and we have the ability to do a lot more customizations
                                    than they do, still being able to serve it in a simple way
                                    and have people be like, wow,
                                    this is far more useful than GPT or something else I'm using,
                                    allows us to be much more of a direct competitor.
                                    So I'm very interested in this being the design issue
                                    that we want to tackle.
                                    Sorry.
                                    Next, it's important for us to design
                                    the kind of intuitive consumer funnels.
                                    This is like more of a retention
                                    and growth research kind of thing,
                                    to figure out, click to click button to button,
                                    what is gonna keep users on?
                                    What is gonna keep users saying,
                                    this is not boring.
                                    This is something that I feel like
                                    is just one seamless flow of action.
                                    And not, oh my God, I have to learn 50 things.
                                    There's 100 buttons here, I don't know what I'm doing.
                                    We want simple funnels.
                                    Another thing that we need to design of course is the,
                                    as we said here, the reasoning and scripting system
                                    for this chat interface.
                                    The back end nodes and architecture links directly to the,
                                    sorry, I gotta fix that link.
                                    Should be linking to this.
                                    Which is the previous node-based technical overview
                                    of what's up in all the nodes that Shannon had built out.
                                    And this is something that we want to of course,
                                    adapt towards the cloud and the chat interface,
                                    but it contains all the technical information
                                    of what we already have available.
                                    So it's very important that we continue to design
                                    on that end and fix that link.
                                    And all you guys, you should have this shared with you now.
                                    So you should be able to look alongside with me,
                                    feel free to click through.
                                    And then finally, we need to design
                                    like the marketing and narrative.
                                    We're working with Riva and working with Mike on this
                                    on making sure that this is something that we can ship
                                    as a story and we can explain to people
                                    the importance of this seamlessness
                                    and show them how useful something like this
                                    can be malleting by example.
                                    A development comes down to this too.
                                    For development, we want to get
                                    those fundamental common nodes down.
                                    The cloud is really useful.
                                    Shannon, I'd love if you want to expand on this
                                    because we can implement any of the ML libraries we want.
                                    We can build just one build for ourselves.
                                    We don't have to try to accommodate
                                    for every single type of system
                                    like we did with the old forge.
                                    - Yeah. - Yeah.
                                    - Yeah, I was gonna say like, I know that like,
                                    this is something that we were discussing
                                    sort of in terms of fort planning.
                                    You can see everyone else that wasn't there.
                                    But like how we were building forge
                                    is it was the tool we personally would want.
                                    But where all people would, we're like, you know,
                                    stacks of like, you know, multiple 40 90s
                                    and $10,000 MacBooks and stuff.
                                    The reality is the average consumer
                                    wouldn't even be able to run like forge.
                                    And even if it was like an agent with like two or three nodes
                                    and it was just like homey 70
                                    and would still just be so slow
                                    and painful on a bad user experience.
                                    So the advantage of moving the cloud is that, you know,
                                    like we can, we can manage all that.
                                    "
                    },
                    {
                        "role": "user",
                        "content": insert
                    }
                ]
            });
            let mut res = client.post("http://localhost:11434/api/chat")
                .json(&request_body)
                .send()
                .await
                .map_err(|e| AppError::Other(e.to_string()))?;
            if res.status().is_success() {
                let mut cum_str = String::new();
                while let Some(chunk) = res.chunk().await.map_err(|e| AppError::Other(e.to_string()))? {
                    let api_response: ApiResponse = serde_json::from_slice(&chunk)?;
                    cum_str.push_str(&api_response.message.content);
                }
                let (finished, unfinished) = parse_topics(&cum_str);
                f = unfinished;
                for i in &finished {
                    total.push(i.clone());
                }
            }else{
                eprintln!("Failed to send request: {}", res.status());
            }
        }
        println!("finished tagging the text into the topic chunks");
        let mut llm = String::new();
        for items in total{
            println!("Passing chunk to be summzarized by LLM");
            let request_body = serde_json::json!({
                "model": "llama3",
                "messages": [
                    {
                        "role": "system",
                        "content": "Your  to take this conversation and sumarize in a clear and concise manner "
                    },
                    {
                        "role": "user",
                        "content": items
                    }
                ]
            });
            let mut res = client.post("http://localhost:11434/api/chat")
                .json(&request_body)
                .send()
                .await
                .map_err(|e| AppError::Other(e.to_string()))?;
            if res.status().is_success() {
                let mut cum_str = String::new();
                while let Some(chunk) = res.chunk().await.map_err(|e| AppError::Other(e.to_string()))? {
                    let api_response: ApiResponse = serde_json::from_slice(&chunk)?;
                    cum_str.push_str(&api_response.message.content);
                }
                // Here is where I need to write ./google_docs.txt with the cum_str to.
                fs::write("/Users/j-supha/Desktop/Personal_AI/FFMPEG/audio/google_docs.txt", &cum_str).expect("unable to write file");
                llm = format!("{:?}\n\n{:?}",llm, cum_str);

            }else{
                eprintln!("Failed to send request: {}", res.status());
            }

        }
        println!("This is the culmination of some hard work");
            let _output = Command::new("python3")
                .arg("google_docs.py")  // Path to the Python script
                .arg("--write")
                .arg(llm)               // Argument to pass to the Python script
                .output()                   // Executes the command as a child process
                .expect("Failed to execute command");
            // 1HFD4EzZqm_i_AUn3NcbI1Bz8rZNRpENqQuB4oNGmbKY this is the document ID

        Ok(())
    }

    // Combined method to process audio file and handle API interaction
    pub async fn process_audio_file(&mut self) -> Result<(), AppError> {
        println!("Processing audio file...");
        let text = self.extract_text_from_audio()?;
        println!("Extracted text from audio...");
        self.send_text_to_api(text).await?;
        Ok(())
    }
}