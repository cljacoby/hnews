use serde::Deserialize;
use serde::Serialize;

pub type Score=u32;
pub type Id=u32;

#[derive(Debug)]
pub struct Date(pub u16, pub u8, pub u8);

#[derive(Debug, Serialize, Deserialize)]
pub struct Listing {
    pub title: String,
    pub id: Id,
    pub score: Option<Score>,
    pub user: Option<String>,
    // comments: u32,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Comment {
    pub user: String,
    pub id: Id,
    pub text: String,
    pub indent: u32,
    // pub bool: deleted,
    pub children: Vec<Comment>,
}

pub mod firebase {

    use super::*;

    #[derive(Debug, Serialize, Deserialize)]
    pub struct User {
        /// The user's unique username. Case-sensitive.
        id: Option<String>,
        /// Delay in minutes between a comment's creation and its visibility to other users.
        delay: Option<u32>,
        /// Creation date of the user, in Unix Time.
        created: u32,
        /// The user's karma.
        karma: Score,
        /// The user's optional self-description. HTML.
        about: Option<String>,
        /// List of the user's stories, polls and comments.
        submitted: Option<Vec<Id>>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Job {
        /// The item's unique id.
        pub id: Id,
        /// true if the item is deleted.
        pub deleted: Option<bool>,
        /// The username of the item's author.
        pub by: Option<String>,
        /// Creation date of the item, in Unix Time.
        pub time: u64,
        /// true if the item is dead.
        pub dead: Option<bool>,
        /// The ids of the item's comments, in ranked display order.
        pub kids: Option<Vec<Id>>,
        /// The comment, story or poll text. HTML.
        pub text: Option<String>,
        /// The URL of the story.
        pub url: Option<String>,
        /// The title of the story, poll or job.
        pub title: String,
    }

    // TODO: This is essentially a Listing, at least with respect to what it represents in the data
    // model. There should be some sort of unification in the API.
    #[derive(Serialize, Deserialize, Debug)]
    pub struct Story {
        /// The item's unique id.
        pub id: Id,
        /// True if the item is deleted.
        pub deleted: Option<bool>,
        /// The type of item. One of "job", "story", "comment", "poll", or "pollopt".
        pub by: Option<String>,
        /// Creation date of the item, in Unix Time.
        pub time: u64,
        /// True if the item is dead.
        pub dead: Option<bool>,
        /// The ids of the item's comments, in ranked display order.
        pub kids: Option<Vec<Id>>,
        /// In the case of stories or polls, the total comment count.
        pub descendants: Option<u32>,
        /// The story's score, or the votes for a pollopt.
        pub score: Option<Score>,
        /// The title of the story, poll or job.
        pub title: Option<String>,
        /// The URL of the story.
        pub url: Option<String>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Comment {
        // Fields directly obtained from the response payload
        /// The item's unique id.
        pub id: Id,
        /// true if the item is deleted.
        pub deleted: Option<bool>,
        /// The type of item. One of "job", "story", "comment", "poll", or "pollopt".
        pub by: Option<String>,
        /// Creation date of the item, in Unix Time.
        pub time: u64,
        /// true if the item is dead.
        pub dead: Option<bool>,
        /// The ids of the item's comments, in ranked display order.
        pub kids: Option<Vec<Id>>,
        /// The comment's parent: either another comment or the relevant story.
        pub parent: Option<Id>,
        /// The comment, story or poll text. HTML.
        pub text: Option<String>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Poll {
        /// The item's unique id.
        pub id: Id,
        /// true if the item is deleted.
        pub deleted: Option<bool>,
        /// The type of item. One of "job", "story", "comment", "poll", or "pollopt".
        pub by: Option<String>,
        /// Creation date of the item, in Unix Time.
        pub time: u64,
        /// true if the item is dead.
        pub dead: Option<bool>,
        /// The ids of the item's comments, in ranked display order.
        pub kids: Option<Vec<Id>>,
        /// A list of related pollopts, in display order.
        pub parts: Option<Vec<Id>>,
        /// In the case of stories or polls, the total comment count.
        pub descendants: Option<u32>,
        /// The story's score, or the votes for a pollopt.
        pub score: Option<Score>,
        /// The title of the story, poll or job.
        pub title: Option<String>,
        /// The comment, story or poll text. HTML.
        pub text: Option<String>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct PollOption {
        /// The item's unique id.
        pub id: Id,
        /// true if the item is deleted.
        pub deleted: Option<bool>,
        /// The type of item. One of "job", "story", "comment", "poll", or "pollopt".
        pub by: Option<String>,
        /// Creation date of the item, in Unix Time.
        pub time: u64,
        /// true if the item is dead.
        pub dead: Option<bool>,
        /// The ids of the item's comments, in ranked display order.
        pub kids: Option<Vec<Id>>,
        /// The comment's parent: either another comment or the relevant story.
        pub parent: Option<Id>,
        /// The story's score, or the votes for a pollopt.
        pub score: Option<Score>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(tag = "type")]
    pub enum Item {
        #[serde(rename = "job")]
        Job(Job),
        #[serde(rename = "story")]
        Story(Story),
        #[serde(rename = "comment")]
        Comment(Comment),
        #[serde(rename = "poll")]
        Poll(Poll),
        #[serde(rename = "pollopt")]
        PollOption(PollOption),
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub(crate) struct ItemsAndProfiles {
        pub items: Vec<Id>,
        pub profiles: Vec<String>,
    }
    
    impl Item {
        pub fn is_job(&self) -> bool {
            match self {
                Self::Job(_job) => true,
                _ => false,
            }
        }

        pub fn is_story(&self) -> bool {
            match self {
                Self::Story(_story) => true,
                _ => false,
            }
        }
        
        pub fn is_comment(&self) -> bool {
            match self {
                Self::Comment(_comment) => true,
                _ => false,
            }
        }

        pub fn is_poll(&self) -> bool {
            match self {
                Self::Poll(_poll) => true,
                _ => false,
            }
        }
        
        pub fn is_poll_option(&self) -> bool {
            match self {
                Self::PollOption(_poll_opt) => true,
                _ => false,
            }
        }
    
    }

}


#[cfg(test)]
mod tests {

    use super::firebase::Story;
    use super::firebase::Item;

    #[test]
    fn test_item_type() {
        let story = Item::Story(Story { id: 27476206,
            deleted: None,
            by: Some("what_ever".to_string()),
            time: 1623432780,
            dead: None,
            kids: Some(vec![27488169, 27478163, 27488195, 27477211, 27488706, 27477425, 27477221, 27489125, 27490162, 27489280, 27487982, 27479605, 27490009, 27488234, 27491642, 27489141, 27477380, 27489264]),
            descendants: Some(314),
            score: Some(529),
            title: Some("Apple admits it ranked its Files app ahead of competitor Dropbox".to_string()),
            url: Some("https://www.theverge.com/2021/6/11/22528701/apple-rank-own-app-over-competitor-files-dropbox-wwdc-2017".to_string())
        });
    
        assert!(story.is_story());
    }



}
