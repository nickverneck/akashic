##AI video Editor
### introduction
create a system that leverage machine learning algorithms like computer vision , audio to text context awareness , LLMs to speed up the process of editing footage into complete videos r comercial use, archiving, youtube .

### how it would work ?

For interviews where audio is king we use a audio to text model to transcribe the audio while indentifying pauses , then we can create a metadata file that contains the trascribed text both for subtitles, cutting, editing. we can develop a system that can also use Davinci Resolve XML to import the videos already precut . or use FFMPEG to create the cut version without the use of an external software.

for B-rolls , we use computer vision models that can categorize and log the broll for further use later. for example the metadata might already contain what camera and lens was used but might not say if it was filmed at night, day , what is the subject and so on. with the extra log of information a editor can easily search for footage based on its contents instead of looking through each footage themselves.

for constructing the edit, we can use a LLM that receives from a USER a basic idea description of how he wants the video. for example the user might wanna create social media vertical shorts for a already made video which would require to cut the most interesting bits into short format videos. or even connect the A-roll of the interview with already cutted down pauses and uniniteresting bits to the Broll showing the footage related  to the A-roll by using the metadata created by the vision model . This creates a easy way to use NPL to speedup the creative workflow .

### technologies 

 -FRONTEND - either a PWA or native  using something like tauri 2. we want the most amount of users to be able to use in different devices , from workstations to small clients like ipads or vr headsets

model serving - on an Ideal scenario everything would be served locally , the CV,ATT,LLM . more research would have to be done to deliver everything local as there are capable models 
that can run locally on small devices like a apple macbook pro however for phones , vr headsets we might still need to be served by cloud.
for that we might run a hybrid where the code is open source and users can host themselves for free or they can pay for our service to host all the infrastucture for them with more powerful models that can serve through cloud and allow them to run everything in any device they desire.

  -BACKEND -let's build the backend in python for the easy of use and easy for people to collaborate, however on our CI/CD pipeline let's add so that everything that is critical to be rebuilt in rust for a memory safe perfomant backend that can run easily on different devices.

 Database - for optimal use of metadata we might wanna use a rag system or GraphRag . that can help smaller llms create better videos by getting the relationship of each video in a more coherent form.

Video editing tools: premiere, Davinci resolve both accept XML files. ffmpeg can be used to cutting videos in to specific formats without having to depend on external tools for simpler edits and automations where a user might wanna send footage to be processed and automatically uploaded to a place for further uses.

### AI Models 

 -- LLMS: 
 - llama3.2 5b seems to be a good mix of being small enough that be run locally in multiple devices but also coherent enough to be molded into the tool that we need by extending its capbilities with rag ,Knowledge graphs or possible some extra Reiforcmenet learning.
- Gemma3 3b seems super light however on basic tests and it seems to be necessary good prompting or perhaps some RL training to get it to do things outside of its scope.
- gema3 250M is extremely small, can probably be run out of a cellphone or raspberry PI but will need further training to create coherent outputs.
- gemini2,5 flash is cloud only, its super faster but might need to be further investigated with RAG and KG to see if its a viable option.

 -- Audio to text
 - Whisper from openAI might be a contender , needs further testing.
 - mordern webbrowsers actually have a simple model that works pretty decently so this could be processed in the browser itself.
 *processing audio in real time might be problematic. a 2h audio might take over 2h to be processed and transcribed , further research for speeding up such audios needs to occur.
 for example  speeding up the audio 2x to cut down the time in half . it might create side effects like wrong words being trascribed due to the change in pitch.

 -- Vision models
    llava model could be used but further investigation is necessary
    claude is cloud only but extremely reliable
    gemini is cloud only but further investigation is necessary
    gpt is cloud only but futher investigation is necessary

